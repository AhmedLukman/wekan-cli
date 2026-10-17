use super::admin_fixtures as fixtures;
use super::support::*;
use reqwest::Method;
use serde_json::{Value, json};

type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Value {
    let mut argv = vec!["wekan", "--server", server, "--output", "json"];
    argv.extend_from_slice(args);
    let success = app
        .execute(Cli::try_parse_from(argv).unwrap().command)
        .await
        .unwrap();
    serde_json::from_str::<Value>(&wekan_cli::output::render_success(
        OutputFormat::Json,
        &success,
    ))
    .unwrap()["data"]
        .clone()
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v11.06 stack and WEKAN_AUDIT_E2E_URL"]
async fn audit_regressions_match_wekan_v11_06() {
    let server = env::var("WEKAN_AUDIT_E2E_URL")
        .expect("set WEKAN_AUDIT_E2E_URL to a fresh isolated v11.06 stack");
    let canonical = ServerUrl::parse(&server).unwrap().as_str().to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("compatibility-audit");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Audit-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let registered = run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &format!("audit_{nonce}"),
            "--password-stdin",
        ],
    )
    .await;
    let user = registered["user_id"].as_str().unwrap();
    // Own this isolated fixture account even when rerunning the suite after a failure.
    let script = format!(
        "const r=db.getSiblingDB('wekan').users.updateOne({{_id:{}}},{{$set:{{isAdmin:true}}}});if(r.matchedCount!==1)throw Error('fixture user missing');",
        json!(user)
    );
    let promoted = Command::new("docker")
        .args([
            "compose", "exec", "-T", "mongodb", "mongosh", "--quiet", "--eval", &script,
        ])
        .output()
        .unwrap();
    assert!(promoted.status.success(), "fixture admin promotion failed");
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
    let board_data = run(
        &app,
        &server,
        &["board", "create", "--title", "Audit regressions"],
    )
    .await;
    let board = board_data["board_id"].as_str().unwrap();
    let lane = board_data["default_swimlane_id"].as_str().unwrap();
    let list_data = run(
        &app,
        &server,
        &[
            "list",
            "create",
            "--board",
            board,
            "--title",
            "Watched list",
            "--swimlane-id",
            lane,
        ],
    )
    .await;
    let list = list_data["list_id"].as_str().unwrap();
    let card_data = run(
        &app,
        &server,
        &[
            "card",
            "create",
            "--board",
            board,
            "--list",
            list,
            "--swimlane-id",
            lane,
            "--title",
            "Watched card",
        ],
    )
    .await;
    let card = card_data["card_id"].as_str().unwrap();
    let field = live_api_json(
        &http,
        &canonical,
        &token,
        Method::POST,
        &format!("api/boards/{board}/custom-fields"),
        Some(json!({"name":"Lookup","type":"text","settings":{}})),
    )
    .await;
    let field = required_id(&field, "custom field");
    live_api_json(
        &http,
        &canonical,
        &token,
        Method::PUT,
        &format!("api/boards/{board}/lists/{list}/cards/{card}"),
        Some(json!({"customFields":[{"_id":field,"value":"watched"}]})),
    )
    .await;

    // UI watch/unwatch persists optional arrays. Every full-document REST projection must map them.
    for watching in [None, Some(true), Some(false)] {
        if let Some(watching) = watching {
            tokio::time::timeout(std::time::Duration::from_secs(30), async {
                let mut session = ddp::Session::login(&server, &token).await;
                for (collection, id) in [("cards", card), ("lists", list)] {
                    let modifier = if watching {
                        json!({"$addToSet":{"watchers":user}})
                    } else {
                        json!({"$pull":{"watchers":user}})
                    };
                    assert_eq!(
                        session
                            .call(
                                &format!("/{collection}/update"),
                                json!([{"_id":id},modifier])
                            )
                            .await,
                        json!(1)
                    );
                }
                session.close().await;
            })
            .await
            .expect("watch fixture timed out");
        }
        let expected = match watching {
            None => Value::Null,
            Some(true) => json!([user]),
            Some(false) => json!([]),
        };
        for args in [
            vec!["card", "get", card],
            vec!["card", "get", card, "--board", board, "--list", list],
            vec!["list", "get", list, "--board", board],
        ] {
            assert_eq!(run(&app, &server, &args).await["watchers"], expected);
        }
        let query = run(
            &app,
            &server,
            &[
                "card",
                "find-by-custom-field",
                "--board",
                board,
                "--custom-field",
                &field,
                "--value",
                "watched",
            ],
        )
        .await;
        assert_eq!(query["cards"].as_array().unwrap().len(), 1);
        assert_eq!(query["cards"][0]["watchers"], expected);
    }

    let integration = run(
        &app,
        &server,
        &[
            "integration",
            "create",
            "--board",
            board,
            "--url",
            "https://example.com/webhook",
        ],
    )
    .await;
    let integration = integration["integration_id"].as_str().unwrap();
    live_api_json(
        &http,
        &canonical,
        &token,
        Method::PUT,
        &format!("api/boards/{board}/integrations/{integration}"),
        Some(json!({"title":null,"token":null,"enabled":false})),
    )
    .await;
    assert_eq!(
        run(
            &app,
            &server,
            &["integration", "get", integration, "--board", board]
        )
        .await["title"],
        Value::Null
    );
    assert_eq!(
        run(&app, &server, &["integration", "list", "--board", board]).await["integrations"][0]["title"],
        Value::Null
    );

    let definition=json!({"trigger":{"activityType":"audit","description":"Built-in rename description"},"action":{"actionType":"audit"}}).to_string();
    let rule = run(
        &app,
        &server,
        &["rule", "create", "--board", board, "--rule", &definition],
    )
    .await;
    let rule = rule["rule_id"].as_str().unwrap();
    assert_eq!(
        run(&app, &server, &["rule", "get", rule, "--board", board]).await["trigger"]["description"],
        "Built-in rename description"
    );
    run(
        &app,
        &server,
        &[
            "rule",
            "update",
            rule,
            "--board",
            board,
            "--rule",
            r#"{"trigger":{"description":null}}"#,
        ],
    )
    .await;
    assert_eq!(
        run(&app, &server, &["rule", "get", rule, "--board", board]).await["trigger"]["description"],
        Value::Null
    );

    // Explicit nulls survive JSON updates and typed reads; the required auth method stays valid.
    let mut settings = json!({"defaultAuthenticationMethod":"password"});
    for (wire, _, _, _) in fixtures::GLOBAL_FIELDS {
        if *wire != "defaultAuthenticationMethod" {
            settings[*wire] = Value::Null;
        }
    }
    let settings_file = directory.path().join("null-settings.json");
    fs::write(&settings_file, settings.to_string()).unwrap();
    let updated = run(
        &app,
        &server,
        &[
            "admin",
            "settings",
            "update",
            "--settings-file",
            settings_file.to_str().unwrap(),
        ],
    )
    .await;
    assert_eq!(updated["submitted_fields"].as_array().unwrap().len(), 35);
    assert_eq!(updated["settings"]["product_name"], Value::Null);
    let raw = live_api_json(&http, &canonical, &token, Method::GET, "api/settings", None).await;
    for (wire, _, _, _) in fixtures::GLOBAL_FIELDS {
        if *wire != "defaultAuthenticationMethod" {
            assert_eq!(raw[*wire], Value::Null, "{wire}");
        }
    }
    assert_eq!(
        run(&app, &server, &["admin", "settings", "get"]).await["settings"]["product_name"],
        Value::Null
    );

    // Administrative creation always sends both keys, even for an omitted identity complement.
    let username = format!("audit_username_only_{nonce}");
    let created = run(
        &app,
        &server,
        &[
            "user",
            "create",
            "--username",
            &username,
            "--password-stdin",
        ],
    )
    .await;
    assert_eq!(created["creation_requested"], true);
    assert_eq!(created["mutation_confirmed"], false);
    assert!(created.get("created").is_none());
    wait_for_live_user(&app, &server, &username).await;
    let before = run(&app, &server, &["user", "list"]).await;
    let duplicate = run(
        &app,
        &server,
        &[
            "user",
            "create",
            "--username",
            &username,
            "--password-stdin",
        ],
    )
    .await;
    assert_eq!(duplicate["mutation_confirmed"], false);
    assert_eq!(
        run(&app, &server, &["user", "list"]).await["users"],
        before["users"]
    );
    let email = format!("audit_email_only_{nonce}@example.test");
    run(
        &app,
        &server,
        &["user", "create", "--email", &email, "--password-stdin"],
    )
    .await;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let users = run(&app, &server, &["user", "list"]).await;
        let mut found = false;
        for candidate in users["users"].as_array().unwrap() {
            if before["users"]
                .as_array()
                .unwrap()
                .iter()
                .any(|u| u["user_id"] == candidate["user_id"])
            {
                continue;
            }
            let detail = run(
                &app,
                &server,
                &["user", "get", candidate["user_id"].as_str().unwrap()],
            )
            .await;
            if detail["emails"][0]["address"] == email {
                assert_eq!(detail["username"], Value::Null);
                found = true;
                break;
            }
        }
        if found {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "email-only creation did not persist"
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    let missing = app
        .execute(
            Cli::try_parse_from([
                "wekan", "comment", "get", "missing", "--board", board, "--card", card,
            ])
            .unwrap()
            .command,
        )
        .await
        .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::NotFound);
    assert_eq!(missing.details().http_status, Some(200));
    assert_eq!(missing.details().wekan_status_code, None);
    assert_eq!(missing.details().server_error, None);
    assert_eq!(missing.details().server_reason, None);

    // Keep the pinned list-copy failure observable; the CLI must not supply a hidden destination.
    let failure = http
        .post(format!("{canonical}api/boards/{board}/lists/{list}/copy"))
        .bearer_auth(&token)
        .json(&json!({"toBoardId":board}))
        .send()
        .await
        .unwrap();
    assert_eq!(failure.status(), 500);
    assert!(
        failure.headers()["content-type"]
            .to_str()
            .unwrap()
            .contains("text/html")
    );
    for (method, path, body) in [
        (Method::POST, "api/deletetoken", json!({"userId":4})),
        (Method::PUT, "api/admin/attachment-settings", json!([])),
    ] {
        let result = live_api_json(&http, &canonical, &token, method, path, Some(body)).await;
        assert_eq!(result["errorType"], "Match.Error");
        assert_eq!(result["path"], "");
        assert_eq!(result["sanitizedError"]["error"], 400);
        assert!(result.get("error").is_none());
    }
}
