use std::sync::Arc;

use serde_json::json;
use sha2::{Digest, Sha256};

use super::support::*;

#[derive(Clone)]
struct Secrets(Arc<Mutex<SecretString>>);

impl SecretInputProvider for Secrets {
    fn read_raw_secret(&self) -> Result<SecretString, AppError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn read_new_account_password(&self, _: bool) -> Result<SecretString, AppError> {
        self.read_raw_secret()
    }
    fn read_login_secrets(&self, _: LoginSecretMode) -> Result<LoginSecrets, AppError> {
        Ok(LoginSecrets::new(self.read_raw_secret()?, None))
    }
}

async fn run(app: &App<CapturingStore, Secrets>, server: &str, args: &[&str]) -> CommandSuccess {
    let mut argv = vec!["wekan", "--server", server, "--output", "json"];
    argv.extend_from_slice(args);
    app.execute(Cli::try_parse_from(argv).unwrap().command)
        .await
        .unwrap()
}

fn seed_workspace_tree(user_id: &str, tree: &serde_json::Value) {
    // v11.06 models/users.js:539-553 stores nodes as blackbox objects;
    // server/models/users.js:761-765 persists setWorkspacesTree without filtering them.
    // Seed only this isolated test's account, then use Wekan's real REST serializer.
    let selector = json!({"_id":user_id});
    let update = json!({"$set":{"profile.boardWorkspacesTree":tree}});
    let script = format!(
        "const result = db.getSiblingDB('wekan').users.updateOne({selector}, {update}); \
         if (result.matchedCount !== 1) throw new Error('test user not found');"
    );
    let result = Command::new("docker")
        .args([
            "compose", "exec", "-T", "mongodb", "mongosh", "--quiet", "--eval", &script,
        ])
        .output()
        .expect("Docker Compose must be available for workspace fixtures");
    assert!(
        result.status.success(),
        "workspace fixture failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

async fn verify_workspace_tree_decoding(
    app: &App<CapturingStore, Secrets>,
    server: &str,
    user_id: &str,
) {
    let canonical = ServerUrl::parse(server).unwrap().as_str().to_owned();
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let http = reqwest::Client::new();
    let known = json!([{
        "id":"root", "name":"Delivery", "children":[
            {"id":"child", "name":"Nested", "icon":"folder", "children":[]}, {}
        ]
    }]);
    for tree in [known.clone(), json!([])] {
        seed_workspace_tree(user_id, &tree);
        let raw = live_api_json(
            &http,
            &canonical,
            &token,
            reqwest::Method::GET,
            "api/user",
            None,
        )
        .await;
        assert_eq!(raw["profile"]["boardWorkspacesTree"], tree);
        for args in [vec!["user", "current"], vec!["user", "get", user_id]] {
            let detail = match run(app, server, &args).await {
                CommandSuccess::UserCurrent(detail) | CommandSuccess::UserShown(detail) => detail,
                _ => panic!("expected typed user detail"),
            };
            let nodes = detail.profile.board_workspaces_tree.unwrap();
            assert_eq!(nodes.len(), tree.as_array().unwrap().len());
            if !nodes.is_empty() {
                assert_eq!(nodes[0].id.as_deref(), Some("root"));
                assert_eq!(nodes[0].name.as_deref(), Some("Delivery"));
                assert_eq!(nodes[0].icon, None);
                let children = nodes[0].children.as_ref().unwrap();
                assert_eq!(children[0].icon.as_deref(), Some("folder"));
                assert_eq!(children[0].children.as_ref().unwrap().len(), 0);
                assert_eq!(children[1].id, None);
                assert_eq!(children[1].name, None);
                assert_eq!(children[1].children, None);
            }
        }
    }
    for nested in [false, true] {
        let mut unknown = known.clone();
        if nested {
            unknown[0]["children"][0]["unmapped"] = json!(true);
        } else {
            unknown[0]["unmapped"] = json!(true);
        }
        seed_workspace_tree(user_id, &unknown);
        let raw = live_api_json(
            &http,
            &canonical,
            &token,
            reqwest::Method::GET,
            "api/user",
            None,
        )
        .await;
        assert_eq!(
            raw["profile"]["boardWorkspacesTree"], unknown,
            "Wekan must preserve the unmapped field for the strict decoder to reject it"
        );
        for args in [
            vec!["user", "current"],
            vec!["user", "get", user_id],
            vec!["auth", "status"],
        ] {
            let mut argv = vec!["wekan", "--server", server, "--output", "json"];
            argv.extend_from_slice(&args);
            let error = app
                .execute(Cli::try_parse_from(argv).unwrap().command)
                .await
                .expect_err(
                    "unmapped workspace fields must fail at the production decoding boundary",
                );
            assert_eq!(error.code(), ErrorCode::ProtocolError);
        }
    }
    seed_workspace_tree(user_id, &known);
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v11.06 stack and WEKAN_COMPATIBILITY_E2E_URL"]
async fn audited_behavior_gaps_match_wekan_v11_06() {
    let server = env::var("WEKAN_COMPATIBILITY_E2E_URL").unwrap();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let username = format!("compat_admin_{nonce}");
    let password = format!("Compat-{nonce}\nsecond line\r\n");
    let secrets = Secrets(Arc::new(Mutex::new(SecretString::from(password.clone()))));
    let directory = TestDirectory::new("compatibility");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".to_owned()), false),
        CapturingStore::default(),
        secrets.clone(),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let CommandSuccess::Registration(registration) = run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &username,
            "--password-raw-stdin",
        ],
    )
    .await
    else {
        panic!("registration")
    };
    let admin_id = registration.user_id;
    run(&app, &server, &["auth", "logout", "--yes"]).await;
    run(
        &app,
        &server,
        &[
            "auth",
            "login",
            "--username",
            &username,
            "--password-raw-stdin",
        ],
    )
    .await;

    let CommandSuccess::UserCurrent(current) = run(&app, &server, &["user", "current"]).await
    else {
        panic!("user")
    };
    assert!(current.profile.date_format.is_some());
    assert!(current.profile.keyboard_shortcuts.is_some());
    assert_eq!(current.created_through_api, None); // REST registration does not set this field.
    let CommandSuccess::UserShown(shown) = run(&app, &server, &["user", "get", &admin_id]).await
    else {
        panic!("user")
    };
    assert_eq!(shown.profile, current.profile);
    let CommandSuccess::UserUpdated(unchanged) =
        run(&app, &server, &["user", "update", &admin_id]).await
    else {
        panic!("update")
    };
    assert_eq!(unchanged.profile, current.profile);
    let CommandSuccess::UserLoginChanged(disabled) = run(
        &app,
        &server,
        &["user", "disable-login", &admin_id, "--yes"],
    )
    .await
    else {
        panic!("self action")
    };
    assert_ne!(disabled.user.login_disabled, Some(true)); // Wekan's self-disable no-op.
    assert_eq!(disabled.user.profile, current.profile);

    verify_workspace_tree_decoding(&app, &server, &admin_id).await;

    let CommandSuccess::BoardCreated(board) = run(
        &app,
        &server,
        &["board", "create", "--title", "Compatibility"],
    )
    .await
    else {
        panic!("board")
    };
    let CommandSuccess::ListCreated(list) = run(
        &app,
        &server,
        &[
            "list",
            "create",
            "--board",
            &board.board_id,
            "--title",
            "Todo",
        ],
    )
    .await
    else {
        panic!("list")
    };
    for (flag, value, expected) in [
        (
            "--wip-limit",
            "5",
            json!({"value":5,"enabled":false,"soft":false}),
        ),
        (
            "--wip-enabled",
            "true",
            json!({"value":1,"enabled":true,"soft":false}),
        ),
        (
            "--wip-soft",
            "true",
            json!({"value":1,"enabled":false,"soft":true}),
        ),
    ] {
        run(
            &app,
            &server,
            &[
                "list",
                "update",
                &list.list_id,
                "--board",
                &board.board_id,
                flag,
                value,
            ],
        )
        .await;
        let CommandSuccess::ListShown(detail) = run(
            &app,
            &server,
            &["list", "get", &list.list_id, "--board", &board.board_id],
        )
        .await
        else {
            panic!("list")
        };
        assert_eq!(serde_json::to_value(detail.wip_limit).unwrap(), expected);
    }
    run(
        &app,
        &server,
        &[
            "list",
            "update",
            &list.list_id,
            "--board",
            &board.board_id,
            "--clear-starred",
        ],
    )
    .await;
    let CommandSuccess::ListShown(detail) = run(
        &app,
        &server,
        &["list", "get", &list.list_id, "--board", &board.board_id],
    )
    .await
    else {
        panic!("list")
    };
    assert_eq!(detail.starred, None);

    for title_args in [vec![], vec!["--title", ""], vec!["--null-title"]] {
        let mut args = vec![
            "card",
            "create",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--swimlane-id",
            &board.default_swimlane_id,
        ];
        args.extend_from_slice(&title_args);
        args.extend_from_slice(&["--member", &admin_id]);
        args.extend_from_slice(&["--null-description", "--due-at", "January 2, 2030"]);
        let CommandSuccess::CardCreated(card) = run(&app, &server, &args).await else {
            panic!("card")
        };
        let CommandSuccess::CardShown(detail) = run(
            &app,
            &server,
            &[
                "card",
                "get",
                &card.card_id,
                "--board",
                &board.board_id,
                "--list",
                &list.list_id,
            ],
        )
        .await
        else {
            panic!("card")
        };
        assert_eq!(
            detail.title,
            if title_args.contains(&"--null-title") {
                None
            } else {
                Some(String::new())
            }
        );
        assert_eq!(detail.description, None);
        assert_eq!(detail.due_at.as_deref(), Some("2030-01-02T00:00:00.000Z"));
        run(&app, &server, &["card", "update", &card.card_id, "--board", &board.board_id, "--list", &list.list_id,
            "--description", " ", "--requested-by", " ", "--assigned-by", " ", "--title", " ",
            "--custom-fields", r#"[{}, {"_id":"object","value":{"key":true}}, {"_id":"mixed","value":[1,true,null,{"x":1},"word"]}]"#,
            "--labels", "[null]", "--locations", r#"[{"latitude":"1.25","longitude":true}]"#,
            "--stickers", r#"[{"position":"2"}]"#, "--due-at", "253402300800000"]).await;
        let CommandSuccess::CardShown(detail) = run(
            &app,
            &server,
            &[
                "card",
                "get",
                &card.card_id,
                "--board",
                &board.board_id,
                "--list",
                &list.list_id,
            ],
        )
        .await
        else {
            panic!("readback")
        };
        assert_eq!(detail.custom_fields[0].custom_field_id.as_deref(), Some(""));
        assert_eq!(detail.label_ids, vec![None]);
        assert_eq!(
            detail.due_at.as_deref(),
            Some("+010000-01-01T00:00:00.000Z")
        );
        assert_eq!(
            detail.locations[0].latitude,
            Some(serde_json::Number::from_f64(1.25).unwrap())
        );
        assert_eq!(detail.locations[0].longitude, Some(1.into()));
        assert_eq!(detail.stickers[0].position, Some(2.into()));
        assert!(detail.description.as_deref().is_none_or(str::is_empty));
        assert!(detail.title.as_deref().is_none_or(str::is_empty));
        assert!(detail.requested_by.as_deref().is_none_or(str::is_empty));
        assert!(detail.assigned_by.as_deref().is_none_or(str::is_empty));
        let value = serde_json::to_value(&detail.custom_fields).unwrap();
        assert_eq!(value[1]["value"], "");
        assert_eq!(value[2]["value"], json!(["1", "true", null, "word"]));
    }
    let CommandSuccess::CardCollection(cards) = run(
        &app,
        &server,
        &[
            "card",
            "list",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
        ],
    )
    .await
    else {
        panic!("list cards")
    };
    assert!(
        cards
            .cards
            .iter()
            .all(|card| card.due_at.as_deref() == Some("+010000-01-01T00:00:00.000Z"))
    );
    for args in [
        vec!["--from", "2030-01-02", "--to", "January 3, 2030"],
        vec!["--from", "2030-01-03", "--to", "2030-01-02"],
    ] {
        let mut command = vec!["user", "cards"];
        command.extend(args);
        let CommandSuccess::UserCards(cards) = run(&app, &server, &command).await else {
            panic!("user cards")
        };
        assert!(cards.cards.is_empty());
    }

    let digest_username = format!("compat_digest_{nonce}");
    let digest = format!("{:x}", Sha256::digest(password.as_bytes()));
    *secrets.0.lock().unwrap() =
        SecretString::from(json!({"digest":digest,"algorithm":"sha-256"}).to_string());
    run(
        &app,
        &server,
        &[
            "user",
            "create",
            "--username",
            &digest_username,
            "--email",
            &format!("{digest_username}@example.test"),
            "--password-digest-stdin",
        ],
    )
    .await;
    // The pinned create-user route returns before account creation finishes.
    let mut created_id = None;
    for _ in 0..40 {
        let CommandSuccess::UserList(users) = run(&app, &server, &["user", "list"]).await else {
            panic!("users")
        };
        if let Some(user) = users
            .users
            .into_iter()
            .find(|user| user.username.as_deref() == Some(&digest_username))
        {
            created_id = Some(user.user_id);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    assert!(
        created_id.is_some(),
        "asynchronous digest user creation must finish"
    );
    let http = reqwest::Client::new();
    let login: serde_json::Value = http
        .post(format!("{server}/users/login"))
        .json(&json!({"username":digest_username,"password":password}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        login["token"].is_string(),
        "digest account must authenticate with the original password"
    );
    let digest_id = login["id"].as_str().unwrap();
    let CommandSuccess::UserShown(digest_user) =
        run(&app, &server, &["user", "get", digest_id]).await
    else {
        panic!("digest user")
    };
    assert_eq!(digest_user.user_id, created_id.unwrap());
    let CommandSuccess::BoardCreated(restricted) = run(
        &app,
        &server,
        &[
            "board",
            "create",
            "--title",
            "Non-admin owner",
            "--owner",
            digest_id,
            "--is-admin",
            "false",
        ],
    )
    .await
    else {
        panic!("owner")
    };
    let token = login["token"].as_str().unwrap();
    let detail: serde_json::Value = http
        .get(format!("{server}/api/boards/{}", restricted.board_id))
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(detail["members"][0]["isAdmin"], false);
    let CommandSuccess::BoardCreated(inactive) = run(
        &app,
        &server,
        &[
            "board",
            "create",
            "--title",
            "Inactive owner",
            "--owner",
            digest_id,
            "--is-active",
            "false",
        ],
    )
    .await
    else {
        panic!("owner")
    };
    let denied = http
        .get(format!("{server}/api/boards/{}", inactive.board_id))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    let status = denied.status();
    let body: serde_json::Value = denied.json().await.unwrap();
    assert!(status == 403 || body["statusCode"] == 403);

    let admin_token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let added = http
        .post(format!(
            "{server}/api/boards/{}/members/{digest_id}/add",
            board.board_id
        ))
        .bearer_auth(&admin_token)
        .json(&json!({"action":"add","role":"admin"}))
        .send()
        .await
        .unwrap();
    assert!(added.status().is_success());
    // With another active administrator, self takeOwnership can deactivate the caller.
    run(
        &app,
        &server,
        &["user", "take-ownership", &admin_id, "--yes"],
    )
    .await;
    let CommandSuccess::BoardShown(after_self) =
        run(&app, &server, &["board", "get", &board.board_id]).await
    else {
        panic!("board")
    };
    let membership = after_self
        .members
        .iter()
        .find(|member| member.user_id == admin_id)
        .unwrap();
    assert!(!membership.is_active);
    assert!(!membership.is_admin);

    // Empty passwords are accepted by registration, although later login is unavailable upstream.
    run(&app, &server, &["auth", "logout", "--yes"]).await;
    *secrets.0.lock().unwrap() = SecretString::from(String::new());
    run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &format!("compat_empty_{nonce}"),
            "--password-raw-stdin",
        ],
    )
    .await;
    run(&app, &server, &["user", "current"]).await;
    run(&app, &server, &["auth", "logout", "--yes"]).await;
    http.post(format!("{server}/users/logout"))
        .bearer_auth(token)
        .json(&json!({"all":true}))
        .send()
        .await
        .unwrap();

    // Exercise production stdin through EOF and native credentials, rather than only the injected provider.
    let config = TestDirectory::new("compatibility-process");
    let _cleanup = NativeCredentialCleanup {
        config_directory: config.path().to_owned(),
        server: server.clone(),
    };
    let process_username = format!("compat_process_{nonce}");
    let register = run_production_cli(
        config.path(),
        &server,
        &[
            "register",
            "--username",
            &process_username,
            "--password-raw-stdin",
        ],
        Some(&password),
    );
    success_json(&register);
    assert_secret_absent(&register, &password);
    success_json(&run_production_cli(
        config.path(),
        &server,
        &["logout"],
        None,
    ));
    let login = run_production_cli(
        config.path(),
        &server,
        &[
            "login",
            "--username",
            &process_username,
            "--password-raw-stdin",
        ],
        Some(&password),
    );
    success_json(&login);
    assert_secret_absent(&login, &password);
    success_json(&run_production_cli(
        config.path(),
        &server,
        &["logout"],
        None,
    ));
    success_json(&run_production_cli(
        config.path(),
        &server,
        &[
            "register",
            "--username",
            &format!("compat_process_empty_{nonce}"),
            "--password-raw-stdin",
        ],
        Some(""),
    ));
    success_json(&run_production_cli(
        config.path(),
        &server,
        &["logout"],
        None,
    ));
}
