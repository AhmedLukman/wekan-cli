use super::support::*;
use serde_json::{Value, json};

type LiveApp = App<CapturingStore, FixedPassword, FileProfileStore>;

async fn run(app: &LiveApp, server: &str, args: &[&str]) -> Result<Value, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output", "json"];
    argv.extend_from_slice(args);
    let success = app
        .execute(Cli::try_parse_from(argv).unwrap().command)
        .await?;
    Ok(
        serde_json::from_str::<Value>(&wekan_cli::output::render_success(
            OutputFormat::Json,
            &success,
        ))
        .unwrap()["data"]
            .clone(),
    )
}

async fn board_member(
    app: &LiveApp,
    server: &str,
    board: &str,
    args: &[&str],
) -> Result<Value, AppError> {
    let mut argv = vec!["board", "member"];
    argv.extend_from_slice(args);
    argv.extend(["--board", board]);
    run(app, server, &argv).await
}

async fn member(app: &LiveApp, server: &str, board: &str, user: &str) -> Value {
    board_member(app, server, board, &["list"]).await.unwrap()["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["user_id"] == user)
        .unwrap()
        .clone()
}

struct CardScope<'a> {
    app: &'a LiveApp,
    server: &'a str,
    board: &'a str,
    list: &'a str,
    card: &'a str,
}
impl CardScope<'_> {
    async fn detail(&self) -> Value {
        run(
            self.app,
            self.server,
            &[
                "card", "get", self.card, "--board", self.board, "--list", self.list,
            ],
        )
        .await
        .unwrap()
    }

    async fn membership(&self, kind: &str, args: &[&str]) -> Result<Value, AppError> {
        let mut argv = vec!["card", kind];
        argv.extend_from_slice(args);
        argv.extend([
            "--board", self.board, "--list", self.list, "--card", self.card,
        ]);
        run(self.app, self.server, &argv).await
    }
}

async fn verify_linked_card_memberships(
    source: &CardScope<'_>,
    swimlane: &str,
    admin: &str,
    user: &str,
) {
    let created = run(
        source.app,
        source.server,
        &[
            "card",
            "create",
            "--board",
            source.board,
            "--list",
            source.list,
            "--swimlane-id",
            swimlane,
            "--linked-id",
            source.card,
        ],
    )
    .await
    .unwrap();
    let linked_id = created["card_id"].as_str().unwrap();
    assert_ne!(linked_id, source.card);
    let link = CardScope {
        card: linked_id,
        ..*source
    };
    let detail = link.detail().await;
    assert_eq!(detail["card_type"], "cardType-linkedCard");
    assert_eq!(detail["linked_id"], source.card);
    assert_eq!(detail["members"], json!([admin]));
    assert_eq!(detail["assignees"], json!([admin]));

    // W11-043: the awaited mutation targets the source, but the response and
    // focused lists read the link's own arrays. Separate detail reads prove it.
    for kind in ["member", "assignee"] {
        let field = if kind == "member" {
            "members"
        } else {
            "assignees"
        };
        for action in ["add", "remove"] {
            let response = link.membership(kind, &[action, user]).await.unwrap();
            assert_eq!(
                response,
                json!({
                    "board_id": source.board,
                    "list_id": source.list,
                    "card_id": linked_id,
                    "user_id": user,
                    "kind": kind,
                    "action": action,
                    "members": [admin],
                    "assignees": [admin],
                })
            );
            let source_detail = source.detail().await;
            assert_eq!(source_detail["card_id"], source.card);
            for array in ["members", "assignees"] {
                let expected = if action == "add" && array == field {
                    json!([admin, user])
                } else {
                    json!([admin])
                };
                assert_eq!(
                    source_detail[array], expected,
                    "source {kind} {action} {array}"
                );
            }
            let link_detail = link.detail().await;
            assert_eq!(link_detail["card_id"], linked_id);
            assert_eq!(link_detail["members"], json!([admin]));
            assert_eq!(link_detail["assignees"], json!([admin]));
            assert_eq!(
                link.membership(kind, &["list"]).await.unwrap()[field],
                json!([admin])
            );
        }
    }
}

const FLAGS: [(&str, &str); 8] = [
    ("--is-admin", "is_admin"),
    ("--is-no-comments", "is_no_comments"),
    ("--is-comment-only", "is_comment_only"),
    ("--is-worker", "is_worker"),
    ("--is-normal-assigned-only", "is_normal_assigned_only"),
    ("--is-comment-assigned-only", "is_comment_assigned_only"),
    ("--is-read-only", "is_read_only"),
    ("--is-read-assigned-only", "is_read_assigned_only"),
];

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v11.06 stack and WEKAN_MEMBERSHIP_E2E_URL"]
async fn complete_membership_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_MEMBERSHIP_E2E_URL")
        .expect("set WEKAN_MEMBERSHIP_E2E_URL to a fresh isolated Wekan v11.06 stack");
    let canonical = ServerUrl::parse(&server).unwrap().as_str().to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let directory = TestDirectory::new("membership-lifecycle");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".into()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(format!("Membership-{nonce}!"))),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let username = format!("wekan_cli_membership_{nonce}");
    let registration = run(
        &app,
        &server,
        &[
            "auth",
            "register",
            "--username",
            &username,
            "--email",
            &format!("{username}@example.test"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap();
    let admin = registration["user_id"].as_str().unwrap().to_owned();
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let board_data = run(
        &app,
        &server,
        &["board", "create", "--title", "Membership lifecycle"],
    )
    .await
    .unwrap();
    let board = board_data["board_id"].as_str().unwrap().to_owned();
    let swimlane = board_data["default_swimlane_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (_normal_dir, normal_app, user) =
        register_live_board_role(&server, &canonical, &token, &board, nonce, "normal").await;
    let (_outsider_dir, outsider_app, outsider) =
        register_live_board_role(&server, &canonical, &token, &board, nonce + 1, "normal").await;
    board_member(&app, &server, &board, &["remove", &outsider])
        .await
        .unwrap();

    // Both mutations acknowledge board summaries even when no membership changes.
    let ack = board_member(&app, &server, &board, &["add", &user, "--role", "worker"])
        .await
        .unwrap();
    assert!(ack.get("mutation_confirmed").is_none());
    assert_eq!(ack["boards"][0]["board_id"], board);
    assert_eq!(
        member(&app, &server, &board, &user).await["is_worker"],
        false
    );
    board_member(&app, &server, &board, &["remove", &admin])
        .await
        .unwrap();
    assert_eq!(
        member(&app, &server, &board, &admin).await["is_active"],
        true
    );

    // Named roles, aliases, case normalization, reactivation, and precedence.
    for (role, enabled) in [
        ("admin", Some("is_admin")),
        ("normal", None),
        ("member", None),
        ("comment", Some("is_comment_only")),
        ("CoMmEnToNlY", Some("is_comment_only")),
        ("readonly", Some("is_read_only")),
        ("worker", Some("is_worker")),
        ("normalassignedonly", Some("is_normal_assigned_only")),
        ("commentassignedonly", Some("is_comment_assigned_only")),
        ("readassignedonly", Some("is_read_assigned_only")),
        ("nocomments", Some("is_no_comments")),
    ] {
        board_member(&app, &server, &board, &["remove", &user])
            .await
            .unwrap();
        assert_eq!(
            member(&app, &server, &board, &user).await["is_active"],
            false
        );
        board_member(
            &app,
            &server,
            &board,
            &["add", &user, "--role", role, "--is-worker", "true"],
        )
        .await
        .unwrap();
        let saved = member(&app, &server, &board, &user).await;
        assert_eq!(saved["is_active"], true);
        for (_, field) in FLAGS {
            assert_eq!(
                saved[field],
                json!(enabled == Some(field)),
                "add {role} {field}"
            );
        }
        let updated = board_member(&app, &server, &board, &["update", &user, "--role", role])
            .await
            .unwrap();
        assert_eq!(updated["affected_documents"], 1);
        assert_eq!(updated["submitted_fields"], json!(["role"]));
        let saved = member(&app, &server, &board, &user).await;
        for (_, field) in FLAGS {
            assert_eq!(
                saved[field],
                json!(enabled == Some(field)),
                "update {role} {field}"
            );
        }
    }
    for (enabled_flag, _) in FLAGS {
        let mut flags = Vec::new();
        for (flag, _) in FLAGS {
            flags.extend([
                flag,
                if flag == enabled_flag {
                    "true"
                } else {
                    "false"
                },
            ]);
        }
        for action in ["add", "update"] {
            if action == "add" {
                board_member(&app, &server, &board, &["remove", &user])
                    .await
                    .unwrap();
            }
            let mut argv = vec![action, &user];
            argv.extend(flags.iter().copied());
            board_member(&app, &server, &board, &argv).await.unwrap();
            let saved = member(&app, &server, &board, &user).await;
            for (flag, field) in FLAGS {
                assert_eq!(
                    saved[field],
                    json!(flag == enabled_flag),
                    "{action} {enabled_flag} {field}"
                );
            }
        }
    }
    board_member(
        &app,
        &server,
        &board,
        &["update", &user, "--role", "normal"],
    )
    .await
    .unwrap();
    // The direct setter defaults omitted assigned-only flags instead of patching.
    board_member(
        &app,
        &server,
        &board,
        &["update", &user, "--role", "normalassignedonly"],
    )
    .await
    .unwrap();
    board_member(
        &app,
        &server,
        &board,
        &["update", &user, "--is-read-only", "true"],
    )
    .await
    .unwrap();
    let partial = member(&app, &server, &board, &user).await;
    assert_eq!(partial["is_read_only"], true);
    assert_eq!(partial["is_normal_assigned_only"], false);
    for role in [
        "normalassignedonly",
        "commentassignedonly",
        "readonly",
        "readassignedonly",
    ] {
        board_member(&app, &server, &board, &["update", &user, "--role", role])
            .await
            .unwrap();
        board_member(
            &app,
            &server,
            &board,
            &["update", &user, "--is-worker", "true"],
        )
        .await
        .unwrap();
        let partial = member(&app, &server, &board, &user).await;
        assert_eq!(partial["is_worker"], true);
        for field in [
            "is_normal_assigned_only",
            "is_comment_assigned_only",
            "is_read_only",
            "is_read_assigned_only",
        ] {
            assert_eq!(partial[field], false, "omitted {role} {field}");
        }
    }
    board_member(
        &app,
        &server,
        &board,
        &["update", &user, "--role", "normal"],
    )
    .await
    .unwrap();
    board_member(
        &app,
        &server,
        &board,
        &["update", &admin, "--role", "normal"],
    )
    .await
    .unwrap();
    assert_eq!(
        member(&app, &server, &board, &admin).await["is_admin"],
        true
    );

    for denied_app in [&normal_app, &outsider_app] {
        for args in [
            vec!["add", &outsider],
            vec!["remove", &user],
            vec!["update", &user, "--role", "admin"],
        ] {
            let error = board_member(denied_app, &server, &board, &args)
                .await
                .unwrap_err();
            assert_eq!(error.code(), ErrorCode::PermissionDenied);
            assert_eq!(error.details().http_status, Some(403));
        }
    }
    let missing = board_member(&app, &server, &board, &["add", "missing-user"])
        .await
        .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::ProtocolError);
    assert!(
        serde_json::to_value(missing.details())
            .unwrap()
            .get("outcome_unknown")
            .is_none()
    );
    let missing = board_member(&app, &server, &board, &["remove", "missing-user"])
        .await
        .unwrap_err();
    assert_eq!(missing.code(), ErrorCode::ProtocolError);

    // A disabled known account cannot be reactivated by board member add.
    run(
        &app,
        &server,
        &["user", "disable-login", &outsider, "--yes"],
    )
    .await
    .unwrap();
    let disabled = board_member(&app, &server, &board, &["add", &outsider])
        .await
        .unwrap_err();
    assert_eq!(disabled.details().http_status, Some(400));
    run(&app, &server, &["user", "enable-login", &outsider])
        .await
        .unwrap();

    let list_data = run(
        &app,
        &server,
        &[
            "list",
            "create",
            "--board",
            &board,
            "--title",
            "Todo",
            "--swimlane-id",
            &swimlane,
        ],
    )
    .await
    .unwrap();
    let list = list_data["list_id"].as_str().unwrap().to_owned();
    let card_data = run(
        &app,
        &server,
        &[
            "card",
            "create",
            "--board",
            &board,
            "--list",
            &list,
            "--swimlane-id",
            &swimlane,
            "--title",
            "Assignments",
        ],
    )
    .await
    .unwrap();
    let card = card_data["card_id"].as_str().unwrap().to_owned();
    let scope = CardScope {
        app: &app,
        server: &server,
        board: &board,
        list: &list,
        card: &card,
    };
    for kind in ["member", "assignee"] {
        let field = if kind == "member" {
            "members"
        } else {
            "assignees"
        };
        assert_eq!(
            scope.membership(kind, &["list"]).await.unwrap()[field],
            json!([])
        );
        scope.membership(kind, &["add", &admin]).await.unwrap();
        let normal_scope = CardScope {
            app: &normal_app,
            ..scope
        };
        let merged = normal_scope
            .membership(kind, &["add", &user])
            .await
            .unwrap();
        assert_eq!(merged[field], json!([admin, user]));
        assert_eq!(
            scope.membership(kind, &["add", &user]).await.unwrap()[field],
            merged[field]
        );
        assert_eq!(
            scope.membership(kind, &["list"]).await.unwrap()[field],
            merged[field]
        );
        for rejected in [&outsider, "missing-user"] {
            let error = scope
                .membership(kind, &["add", rejected])
                .await
                .unwrap_err();
            assert_eq!(error.details().http_status, Some(400));
        }
        let removed = scope.membership(kind, &["remove", &user]).await.unwrap();
        assert_eq!(removed[field], json!([admin]));
        assert_eq!(
            scope.membership(kind, &["remove", &user]).await.unwrap()[field],
            removed[field]
        );
        let error = scope
            .membership(kind, &["remove", "missing-user"])
            .await
            .unwrap_err();
        assert_eq!(error.details().http_status, Some(500));
        assert!(
            serde_json::to_value(error.details())
                .unwrap()
                .get("outcome_unknown")
                .is_none()
        );
    }
    verify_linked_card_memberships(&scope, &swimlane, &admin, &user).await;

    // Changes to either array retain the other array.
    let merged = scope.membership("member", &["add", &user]).await.unwrap();
    assert_eq!(merged["assignees"], json!([admin]));
    scope.membership("assignee", &["add", &user]).await.unwrap();
    board_member(&app, &server, &board, &["remove", &user])
        .await
        .unwrap();
    for kind in ["member", "assignee"] {
        scope.membership(kind, &["remove", &user]).await.unwrap();
    }
    board_member(&app, &server, &board, &["add", &user, "--role", "normal"])
        .await
        .unwrap();

    for role in ["commentonly", "worker", "readonly", "nocomments"] {
        board_member(&app, &server, &board, &["update", &user, "--role", role])
            .await
            .unwrap();
        let denied_scope = CardScope {
            app: &normal_app,
            ..scope
        };
        for kind in ["member", "assignee"] {
            for action in ["add", "remove"] {
                let error = denied_scope
                    .membership(kind, &[action, &admin])
                    .await
                    .unwrap_err();
                assert_eq!(
                    error.code(),
                    ErrorCode::PermissionDenied,
                    "{role} {kind} {action}: {error:?}"
                );
            }
        }
    }
    board_member(
        &app,
        &server,
        &board,
        &["update", &user, "--role", "normal"],
    )
    .await
    .unwrap();
    for bad_scope in [
        CardScope {
            list: "missing-list",
            ..scope
        },
        CardScope {
            card: "missing-card",
            ..scope
        },
    ] {
        for kind in ["member", "assignee"] {
            for action in ["add", "remove"] {
                let error = bad_scope
                    .membership(kind, &[action, &admin])
                    .await
                    .unwrap_err();
                assert_eq!(error.code(), ErrorCode::NotFound);
                assert_eq!(error.details().http_status, Some(404));
            }
        }
    }
    run(
        &app,
        &server,
        &["card", "archive", &card, "--board", &board, "--list", &list],
    )
    .await
    .unwrap();
    for kind in ["member", "assignee"] {
        for action in ["add", "remove"] {
            let error = scope.membership(kind, &[action, &admin]).await.unwrap_err();
            assert_eq!(error.code(), ErrorCode::NotFound);
        }
    }
    run(
        &app,
        &server,
        &["card", "restore", &card, "--board", &board, "--list", &list],
    )
    .await
    .unwrap();

    // Ordinary board administrators can manage memberships through the typed CLI.
    board_member(&app, &server, &board, &["update", &user, "--role", "admin"])
        .await
        .unwrap();
    board_member(
        &normal_app,
        &server,
        &board,
        &["add", &outsider, "--role", "normal"],
    )
    .await
    .unwrap();
    board_member(
        &normal_app,
        &server,
        &board,
        &["update", &outsider, "--role", "worker"],
    )
    .await
    .unwrap();
    board_member(&normal_app, &server, &board, &["remove", &outsider])
        .await
        .unwrap();
    run(&app, &server, &["board", "delete", &board, "--yes"])
        .await
        .unwrap();
}
