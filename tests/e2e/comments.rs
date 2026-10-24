use super::support::*;
#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v12.57 stack and WEKAN_COMMENT_E2E_URL"]
async fn complete_comment_lifecycle_matches_wekan_v12_57() {
    let server = env::var("WEKAN_COMMENT_E2E_URL")
        .expect("set WEKAN_COMMENT_E2E_URL to a fresh isolated Wekan v12.57 server URL");
    let canonical_server = ServerUrl::parse(&server)
        .expect("the live-test Wekan URL must be valid")
        .as_str()
        .to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock must be after the Unix epoch")
        .as_millis();
    let username = format!("wekan_cli_comment_admin_{nonce}");
    let email = format!("{username}@example.test");
    let password = format!("Wekan-comment-e2e-{nonce}!");

    let directory = TestDirectory::new("complete-comment-lifecycle");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password)),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let registration = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "register",
        "--username",
        &username,
        "--email",
        &email,
        "--password-stdin",
    ])
    .unwrap();
    let CommandSuccess::Registration(registration) = app
        .execute(registration.command)
        .await
        .expect("the first account on the fresh comment stack must register as administrator")
    else {
        panic!("expected registration output")
    };
    let admin_id = registration.user_id;
    let admin_token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .expect("registration must store the administrator credential")
        .3
        .clone();
    let http = reqwest::Client::new();

    let CommandSuccess::BoardCreated(board) = execute_live_board(
        &app,
        &server,
        &["create", "--title", &format!("Comment lifecycle {nonce}")],
    )
    .await
    .expect("the comment lifecycle board must be created") else {
        panic!("expected create-board output")
    };
    let CommandSuccess::ListCreated(list) = execute_live_list(
        &app,
        &server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--title",
            "Todo",
            "--swimlane-id",
            &board.default_swimlane_id,
        ],
    )
    .await
    .expect("the comment lifecycle list must be created") else {
        panic!("expected create-list output")
    };
    let CommandSuccess::CardCreated(card) = execute_live_card(
        &app,
        &server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--title",
            "Comment target",
            "--swimlane-id",
            &board.default_swimlane_id,
        ],
    )
    .await
    .expect("the comment target card must be created") else {
        panic!("expected card creation output")
    };

    let CommandSuccess::CommentCreated(created) = execute_live_comment(
        &app,
        &server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--card",
            &card.card_id,
            "--text",
            "  Initial comment  ",
        ],
    )
    .await
    .expect("comment creation must succeed") else {
        panic!("expected comment creation output")
    };
    let CommandSuccess::CommentCollection(collection) = execute_live_comment(
        &app,
        &server,
        &["list", "--board", &board.board_id, "--card", &card.card_id],
    )
    .await
    .expect("the created comment must be listed") else {
        panic!("expected comment collection output")
    };
    let summary = collection
        .comments
        .iter()
        .find(|comment| comment.comment_id == created.comment_id)
        .expect("the collection must contain the created comment");
    assert_eq!(summary.text, "Initial comment");
    assert_eq!(summary.author_id, admin_id);

    let CommandSuccess::CommentShown(detail) = execute_live_comment(
        &app,
        &server,
        &[
            "get",
            "--board",
            &board.board_id,
            "--card",
            &card.card_id,
            &created.comment_id,
        ],
    )
    .await
    .expect("the created comment must be readable") else {
        panic!("expected comment detail output")
    };
    assert_eq!(detail.board_id, board.board_id);
    assert_eq!(detail.card_id, card.card_id);
    assert_eq!(detail.text, "Initial comment");
    assert_eq!(detail.parent_id, None);
    assert_eq!(detail.sync_comment_id, None);
    super::support::attachment_fixtures::fixture_command(
        "mongodb",
        "mongosh",
        "db.getSiblingDB('wekan').card_comments.updateOne({_id:p.id},{$set:{syncCommentId:'external-comment',webhookResponseRevision:'private-revision',webhookResponsePending:{checksum:'private-receipt'}}});print('{}');",
        &serde_json::json!({"id":created.comment_id}),
    );
    let CommandSuccess::CommentShown(synced) = execute_live_comment(
        &app,
        &server,
        &[
            "get",
            &created.comment_id,
            "--board",
            &board.board_id,
            "--card",
            &card.card_id,
        ],
    )
    .await
    .unwrap() else {
        panic!("synced comment")
    };
    assert_eq!(synced.sync_comment_id.as_deref(), Some("external-comment"));

    let raw_created = live_api_json(
        &http,
        &canonical_server,
        &admin_token,
        reqwest::Method::POST,
        &format!(
            "api/boards/{}/cards/{}/comments",
            board.board_id, card.card_id
        ),
        Some(serde_json::json!({"comment": "  Server-trimmed comment  "})),
    )
    .await;
    let raw_comment_id = required_id(&raw_created, "raw comment creation");
    let CommandSuccess::CommentShown(raw_detail) = execute_live_comment(
        &app,
        &server,
        &[
            "get",
            "--board",
            &board.board_id,
            "--card",
            &card.card_id,
            &raw_comment_id,
        ],
    )
    .await
    .expect("the raw-created comment must be readable through the typed boundary") else {
        panic!("expected raw-created comment detail")
    };
    assert_eq!(raw_detail.text, "Server-trimmed comment");

    let mut whitespace_comment_ids = Vec::new();
    let comment_path = format!(
        "api/boards/{}/cards/{}/comments",
        board.board_id, card.card_id
    );
    for (input, expected) in [
        ("\u{85}", "\u{85}"),
        ("\u{85}Hello\u{85}", "\u{85}Hello\u{85}"),
        (" \u{feff}\u{85}Hello\u{85} \n", "\u{85}Hello\u{85}"),
        ("\u{feff}Hello\u{feff}", "Hello"),
    ] {
        let raw = live_api_json(
            &http,
            &canonical_server,
            &admin_token,
            reqwest::Method::POST,
            &comment_path,
            Some(serde_json::json!({"comment": input})),
        )
        .await;
        let raw_id = required_id(&raw, "raw Unicode comment creation");
        let CommandSuccess::CommentCreated(typed) = execute_live_comment(
            &app,
            &server,
            &[
                "create",
                "--board",
                &board.board_id,
                "--card",
                &card.card_id,
                "--text",
                input,
            ],
        )
        .await
        .expect("Wekan-compatible Unicode comment creation must succeed") else {
            panic!("expected Unicode comment creation output")
        };
        for comment_id in [raw_id, typed.comment_id] {
            let raw_detail = live_api_json(
                &http,
                &canonical_server,
                &admin_token,
                reqwest::Method::GET,
                &format!("{comment_path}/{comment_id}"),
                None,
            )
            .await;
            assert_eq!(raw_detail["text"], expected);
            let CommandSuccess::CommentShown(detail) = execute_live_comment(
                &app,
                &server,
                &[
                    "get",
                    "--board",
                    &board.board_id,
                    "--card",
                    &card.card_id,
                    &comment_id,
                ],
            )
            .await
            .expect("Unicode comment reads must match the live server") else {
                panic!("expected Unicode comment detail")
            };
            assert_eq!(detail.text, expected);
            let CommandSuccess::CommentCollection(collection) = execute_live_comment(
                &app,
                &server,
                &["list", "--board", &board.board_id, "--card", &card.card_id],
            )
            .await
            .expect("listing must accept Wekan's Unicode comment text") else {
                panic!("expected Unicode comment collection")
            };
            assert_eq!(
                collection
                    .comments
                    .iter()
                    .find(|comment| comment.comment_id == comment_id)
                    .expect("the Unicode comment must be listed")
                    .text,
                expected
            );
            whitespace_comment_ids.push(comment_id);
        }
    }
    for blank in ["", " \t\r\n", "\u{feff}", " \u{feff}\u{2028}\u{2029} "] {
        let response = http
            .post(format!("{canonical_server}{comment_path}"))
            .bearer_auth(&admin_token)
            .json(&serde_json::json!({"comment": blank}))
            .send()
            .await
            .expect("the blank-comment probe must reach Wekan");
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
        assert!(
            Cli::try_parse_from([
                "wekan",
                "comment",
                "create",
                "--board",
                &board.board_id,
                "--card",
                &card.card_id,
                "--text",
                blank,
            ])
            .is_err(),
            "CLI and Wekan must reject the same blank text"
        );
    }

    let missing = execute_live_comment(
        &app,
        &server,
        &[
            "get",
            "--board",
            &board.board_id,
            "--card",
            &card.card_id,
            "missing-comment",
        ],
    )
    .await
    .expect_err("a missing comment must use the stable not-found error");
    assert_eq!(missing.code(), ErrorCode::NotFound);
    assert_eq!(missing.details().http_status, Some(200));
    assert_eq!(missing.details().wekan_status_code, None);
    assert_eq!(missing.details().server_error, None);
    assert_eq!(missing.details().server_reason, None);

    let nonexistent_card_id = format!("missing-card-{nonce}");
    let (_boundary_probe_dir, boundary_probe, _) = register_live_board_role(
        &server,
        &canonical_server,
        &admin_token,
        &board.board_id,
        nonce + 10,
        "normal",
    )
    .await;

    for arguments in [
        vec![
            "list",
            "--board",
            &board.board_id,
            "--card",
            &nonexistent_card_id,
        ],
        vec![
            "create",
            "--board",
            &board.board_id,
            "--card",
            &nonexistent_card_id,
            "--text",
            "Missing card regression",
        ],
    ] {
        let actor = if arguments[0] == "create" {
            &boundary_probe
        } else {
            &app
        };
        let error = execute_live_comment(actor, &server, &arguments)
            .await
            .expect_err("a nonexistent card must reject comment listing and insertion");
        assert_eq!(error.details().http_status, Some(404));
        assert_eq!(
            error.details().server_error.as_deref(),
            Some("Card not found")
        );
    }

    let (_normal_directory, normal_app, _) = register_live_board_role(
        &server,
        &canonical_server,
        &admin_token,
        &board.board_id,
        nonce,
        "normal",
    )
    .await;
    let (_read_only_directory, read_only_app, _) = register_live_board_role(
        &server,
        &canonical_server,
        &admin_token,
        &board.board_id,
        nonce,
        "readonly",
    )
    .await;
    let (_comment_directory, comment_only_app, _) = register_live_board_role(
        &server,
        &canonical_server,
        &admin_token,
        &board.board_id,
        nonce,
        "commentonly",
    )
    .await;
    let (_worker_directory, worker_app, _) = register_live_board_role(
        &server,
        &canonical_server,
        &admin_token,
        &board.board_id,
        nonce,
        "worker",
    )
    .await;
    let (_no_comments_directory, no_comments_app, _) = register_live_board_role(
        &server,
        &canonical_server,
        &admin_token,
        &board.board_id,
        nonce,
        "nocomments",
    )
    .await;

    for (role, role_app) in [
        ("normal", &normal_app),
        ("comment-only", &comment_only_app),
        ("worker", &worker_app),
    ] {
        let listing = execute_live_comment(
            role_app,
            &server,
            &["list", "--board", &board.board_id, "--card", &card.card_id],
        )
        .await;
        if role == "normal" {
            listing.unwrap();
        } else {
            assert_eq!(listing.unwrap_err().code(), ErrorCode::PermissionDenied);
        }
        let detail = execute_live_comment(
            role_app,
            &server,
            &[
                "get",
                "--board",
                &board.board_id,
                "--card",
                &card.card_id,
                &created.comment_id,
            ],
        )
        .await;
        if role == "normal" {
            detail.unwrap();
        } else {
            assert_eq!(detail.unwrap_err().code(), ErrorCode::PermissionDenied);
        }
        let CommandSuccess::CommentCreated(role_comment) = execute_live_comment(
            role_app,
            &server,
            &[
                "create",
                "--board",
                &board.board_id,
                "--card",
                &card.card_id,
                "--text",
                &format!("{role} comment"),
            ],
        )
        .await
        .unwrap_or_else(|error| panic!("{role} comment creation must succeed: {error:?}")) else {
            panic!("expected {role} comment creation output")
        };
        let deletion = execute_live_comment(
            role_app,
            &server,
            &[
                "delete",
                "--board",
                &board.board_id,
                "--card",
                &card.card_id,
                &role_comment.comment_id,
                "--yes",
            ],
        )
        .await;
        if role == "normal" {
            deletion.unwrap();
        } else {
            assert_eq!(deletion.unwrap_err().code(), ErrorCode::PermissionDenied);
            execute_live_comment(
                &app,
                &server,
                &[
                    "delete",
                    &role_comment.comment_id,
                    "--board",
                    &board.board_id,
                    "--card",
                    &card.card_id,
                    "--yes",
                ],
            )
            .await
            .unwrap();
        }
    }

    for (role, role_app) in [
        ("read-only", &read_only_app),
        ("no-comments", &no_comments_app),
    ] {
        for arguments in [
            vec!["list", "--board", &board.board_id, "--card", &card.card_id],
            vec![
                "get",
                "--board",
                &board.board_id,
                "--card",
                &card.card_id,
                &created.comment_id,
            ],
            vec![
                "create",
                "--board",
                &board.board_id,
                "--card",
                &card.card_id,
                "--text",
                "Denied comment",
            ],
        ] {
            let result = execute_live_comment(role_app, &server, &arguments).await;
            if arguments[0] == "create" || role == "no-comments" {
                let error = result.unwrap_err();
                assert_eq!(error.code(), ErrorCode::PermissionDenied, "{role}");
                assert_eq!(error.details().http_status, Some(403));
            } else {
                result.unwrap();
            }
        }
    }

    let CommandSuccess::CommentCreated(foreign) = execute_live_comment(
        &normal_app,
        &server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--card",
            &card.card_id,
            "--text",
            "Normal member foreign-delete probe",
        ],
    )
    .await
    .expect("the normal member must create the foreign-delete probe") else {
        panic!("expected normal-member comment creation output")
    };
    let denied_foreign_delete = execute_live_comment(
        &normal_app,
        &server,
        &[
            "delete",
            "--board",
            &board.board_id,
            "--card",
            &card.card_id,
            &created.comment_id,
            "--yes",
        ],
    )
    .await
    .expect_err("a normal member must not delete another author's comment");
    assert_eq!(denied_foreign_delete.code(), ErrorCode::PermissionDenied);
    let CommandSuccess::CommentDeleted(admin_deleted_foreign) = execute_live_comment(
        &app,
        &server,
        &[
            "delete",
            "--board",
            &board.board_id,
            "--card",
            &card.card_id,
            &foreign.comment_id,
            "--yes",
        ],
    )
    .await
    .expect("the board admin may delete another author's comment by default") else {
        panic!("expected board-admin comment deletion output")
    };
    assert_eq!(admin_deleted_foreign.delete_mode, CommentDeleteMode::Hard);

    for comment_id in [created.comment_id, raw_comment_id]
        .into_iter()
        .chain(whitespace_comment_ids)
    {
        let CommandSuccess::CommentDeleted(deleted) = execute_live_comment(
            &app,
            &server,
            &[
                "delete",
                "--board",
                &board.board_id,
                "--card",
                &card.card_id,
                &comment_id,
                "--yes",
            ],
        )
        .await
        .expect("comment cleanup must succeed") else {
            panic!("expected comment deletion output")
        };
        assert!(deleted.deleted);
        assert_eq!(deleted.delete_mode, CommentDeleteMode::Hard);
    }
    let CommandSuccess::CommentCollection(empty_after_delete) = execute_live_comment(
        &app,
        &server,
        &["list", "--board", &board.board_id, "--card", &card.card_id],
    )
    .await
    .expect("the final comment collection must remain readable") else {
        panic!("expected final comment collection")
    };
    assert!(empty_after_delete.comments.is_empty());
}
