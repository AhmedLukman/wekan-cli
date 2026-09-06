use super::support::*;
#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v11.06 stack and WEKAN_COMMENT_E2E_URL"]
async fn complete_comment_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_COMMENT_E2E_URL")
        .expect("set WEKAN_COMMENT_E2E_URL to a fresh isolated Wekan v11.06 server URL");
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
    assert_eq!(missing.details().wekan_status_code, Some(404));

    let nonexistent_card_id = format!("missing-card-{nonce}");
    let CommandSuccess::CommentCollection(empty) = execute_live_comment(
        &app,
        &server,
        &[
            "list",
            "--board",
            &board.board_id,
            "--card",
            &nonexistent_card_id,
        ],
    )
    .await
    .expect("Wekan lists a nonexistent card as an empty comment collection") else {
        panic!("expected empty comment collection")
    };
    assert!(empty.comments.is_empty());
    let CommandSuccess::CommentCreated(orphan) = execute_live_comment(
        &app,
        &server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--card",
            &nonexistent_card_id,
            "--text",
            "Orphan comment defect probe",
        ],
    )
    .await
    .expect("Wekan v11.06 permits comment creation for a nonexistent card") else {
        panic!("expected orphan comment creation output")
    };
    let CommandSuccess::CommentShown(orphan_detail) = execute_live_comment(
        &app,
        &server,
        &[
            "get",
            "--board",
            &board.board_id,
            "--card",
            &nonexistent_card_id,
            &orphan.comment_id,
        ],
    )
    .await
    .expect("the orphan comment must remain readable") else {
        panic!("expected orphan comment detail")
    };
    assert_eq!(orphan_detail.card_id, nonexistent_card_id);
    execute_live_comment(
        &app,
        &server,
        &[
            "delete",
            "--board",
            &board.board_id,
            "--card",
            &nonexistent_card_id,
            &orphan.comment_id,
            "--yes",
        ],
    )
    .await
    .expect("the orphan comment cleanup must succeed");

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

    for (role, role_app) in [("normal", &normal_app), ("read-only", &read_only_app)] {
        execute_live_comment(
            role_app,
            &server,
            &["list", "--board", &board.board_id, "--card", &card.card_id],
        )
        .await
        .unwrap_or_else(|error| panic!("{role} comment listing must succeed: {error:?}"));
        execute_live_comment(
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
        .await
        .unwrap_or_else(|error| panic!("{role} comment lookup must succeed: {error:?}"));
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
        execute_live_comment(
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
        .await
        .unwrap_or_else(|error| panic!("{role} own-comment deletion must succeed: {error:?}"));
    }

    for (role, role_app) in [
        ("comment-only", &comment_only_app),
        ("worker", &worker_app),
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
            let error = execute_live_comment(role_app, &server, &arguments)
                .await
                .unwrap_err();
            assert_eq!(error.code(), ErrorCode::PermissionDenied, "{role}");
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

    for comment_id in [created.comment_id, raw_comment_id] {
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
