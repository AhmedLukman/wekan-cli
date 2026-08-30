use super::support::*;
#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v11.06 stack and WEKAN_BOARD_E2E_URL"]
async fn complete_board_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_BOARD_E2E_URL")
        .expect("set WEKAN_BOARD_E2E_URL to a fresh isolated Wekan v11.06 server URL");
    let canonical_server = ServerUrl::parse(&server)
        .expect("the live-test Wekan URL must be valid")
        .as_str()
        .to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock must be after the Unix epoch")
        .as_millis();
    let admin_username = format!("wekan_cli_board_admin_{nonce}");
    let admin_email = format!("{admin_username}@example.test");
    let owner_username = format!("wekan_cli_board_owner_{nonce}");
    let owner_email = format!("{owner_username}@example.test");
    let password = format!("Wekan-board-e2e-{nonce}!");

    let directory = TestDirectory::new("complete-board-lifecycle");
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password.clone())),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let registration = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "register",
        "--username",
        &admin_username,
        "--email",
        &admin_email,
        "--password-stdin",
    ])
    .unwrap();
    let CommandSuccess::Registration(_) = app
        .execute(registration.command)
        .await
        .expect("the first account on the fresh board stack must register as administrator")
    else {
        panic!("expected registration output")
    };
    let admin_token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .expect("registration must store the administrator credential")
        .3
        .clone();

    let CommandSuccess::UserCreated(created) = execute_live_user(
        &app,
        &server,
        &[
            "create",
            "--username",
            &owner_username,
            "--email",
            &owner_email,
            "--password-stdin",
        ],
    )
    .await
    .expect("the administrator must be able to create a second board owner") else {
        panic!("expected create-user output")
    };
    assert!(created.created);
    let owner_id = wait_for_live_user(&app, &server, &owner_username).await;
    let owner_directory = TestDirectory::new("complete-board-lifecycle-owner");
    let owner_app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password)),
        FileProfileStore::at(owner_directory.path().to_owned()),
    );
    let owner_login = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "login",
        "--username",
        &owner_username,
        "--password-stdin",
    ])
    .unwrap();
    let CommandSuccess::Login(_) = owner_app
        .execute(owner_login.command)
        .await
        .expect("the ordinary board owner must be able to log in")
    else {
        panic!("expected login output")
    };
    let CommandSuccess::BoardCount(baseline) = execute_live_board(&app, &server, &["count"])
        .await
        .expect("the initial board count must succeed")
    else {
        panic!("expected board-count output")
    };

    let private_title = format!("Private lifecycle {nonce}");
    let CommandSuccess::BoardCreated(private_created) = execute_live_board(
        &app,
        &server,
        &[
            "create",
            "--title",
            &private_title,
            "--no-comments",
            "--comment-only",
            "--worker",
        ],
    )
    .await
    .expect("private board creation must succeed") else {
        panic!("expected create-board output")
    };
    assert!(!private_created.board_id.is_empty());
    assert!(!private_created.default_swimlane_id.is_empty());

    let public_title = format!("Public lifecycle {nonce}");
    let CommandSuccess::BoardCreated(public_created) = execute_live_board(
        &app,
        &server,
        &[
            "create",
            "--title",
            &public_title,
            "--owner",
            &owner_id,
            "--permission",
            "public",
            "--color",
            "cleanlight",
        ],
    )
    .await
    .expect("public board creation for an explicit owner must succeed") else {
        panic!("expected create-board output")
    };

    let CommandSuccess::BoardCount(after_create) = execute_live_board(&app, &server, &["count"])
        .await
        .expect("the post-create board count must succeed")
    else {
        panic!("expected board-count output")
    };
    assert_eq!(after_create.private, baseline.private + 1);
    assert_eq!(after_create.public, baseline.public + 1);

    let CommandSuccess::BoardList(active) = execute_live_board(&app, &server, &["list"])
        .await
        .expect("active board listing must succeed")
    else {
        panic!("expected active-board output")
    };
    assert_eq!(
        active.scope,
        wekan_cli::command_result::BoardListScope::Active
    );
    assert!(active.boards.iter().any(|board| {
        board.board_id == private_created.board_id && board.title == private_title
    }));

    let CommandSuccess::BoardList(public) =
        execute_live_board(&app, &server, &["list", "--public"])
            .await
            .expect("public board listing must succeed")
    else {
        panic!("expected public-board output")
    };
    assert_eq!(
        public.scope,
        wekan_cli::command_result::BoardListScope::Public
    );
    assert!(
        public.boards.iter().any(|board| {
            board.board_id == public_created.board_id && board.title == public_title
        })
    );

    let CommandSuccess::BoardShown(private_board) =
        execute_live_board(&app, &server, &["get", &private_created.board_id])
            .await
            .expect("private board retrieval must succeed")
    else {
        panic!("expected board-detail output")
    };
    assert_eq!(private_board.title, private_title);
    assert_eq!(private_board.permission.as_deref(), Some("private"));
    assert_eq!(private_board.color.as_deref(), Some("belize"));
    assert_eq!(private_board.members.len(), 1);
    assert!(private_board.watchers.is_empty());
    assert_eq!(private_board.members[0].is_no_comments, Some(true));
    assert_eq!(private_board.members[0].is_comment_only, Some(true));
    assert_eq!(private_board.members[0].is_worker, Some(true));
    assert_eq!(private_board.subtasks_default_board_id, None);
    assert_eq!(private_board.subtasks_default_list_id, None);
    assert_eq!(private_board.date_settings_default_board_id, None);
    assert_eq!(private_board.date_settings_default_list_id, None);

    let copied_board_id = live_api_json(
        &reqwest::Client::new(),
        &canonical_server,
        &admin_token,
        reqwest::Method::POST,
        &format!("api/boards/{}/copy", private_created.board_id),
        Some(serde_json::json!({"title": format!("Watcher lifecycle {nonce}")})),
    )
    .await
    .as_str()
    .expect("the board-copy response must be the copied board ID")
    .to_owned();
    let CommandSuccess::BoardShown(copied_board) =
        execute_live_board(&app, &server, &["get", &copied_board_id])
            .await
            .expect("copied board retrieval with persisted watcher state must succeed")
    else {
        panic!("expected copied board output")
    };
    assert!(copied_board.watchers.is_empty());

    let CommandSuccess::BoardShown(public_board) =
        execute_live_board(&app, &server, &["get", &public_created.board_id])
            .await
            .expect("public board retrieval must succeed")
    else {
        panic!("expected board-detail output")
    };
    assert_eq!(public_board.title, public_title);
    assert_eq!(public_board.permission.as_deref(), Some("public"));
    assert_eq!(public_board.color.as_deref(), Some("cleanlight"));
    assert_eq!(public_board.members.len(), 1);
    assert_eq!(public_board.members[0].user_id, owner_id);
    assert!(public_board.members[0].is_admin);
    assert!(public_board.members[0].is_active);

    wait_for_live_board_admin_membership(&owner_app, &server, &public_created.board_id).await;
    let CommandSuccess::BoardList(owner_active) =
        execute_live_board(&owner_app, &server, &["list"])
            .await
            .expect("an ordinary user must be able to list their active boards")
    else {
        panic!("expected active-board output")
    };
    assert!(
        owner_active
            .boards
            .iter()
            .any(|board| board.board_id == public_created.board_id)
    );
    let denied_public_list = execute_live_board(&owner_app, &server, &["list", "--public"])
        .await
        .expect_err("Wekan v11.06 restricts the public-board catalog to site admins");
    assert_eq!(denied_public_list.code(), ErrorCode::PermissionDenied);
    let denied_count = execute_live_board(&owner_app, &server, &["count"])
        .await
        .expect_err("Wekan v11.06 restricts aggregate board counts to site admins");
    assert_eq!(denied_count.code(), ErrorCode::PermissionDenied);
    let CommandSuccess::BoardShown(owner_public_board) =
        execute_live_board(&owner_app, &server, &["get", &public_created.board_id])
            .await
            .expect("a board owner must be able to retrieve their board")
    else {
        panic!("expected board-detail output")
    };
    assert_eq!(owner_public_board.title, public_title);

    let denied_get = execute_live_board(&owner_app, &server, &["get", &private_created.board_id])
        .await
        .expect_err("an unrelated ordinary user must not read a private board");
    assert_eq!(denied_get.code(), ErrorCode::PermissionDenied);
    let denied_rename = execute_live_board(
        &owner_app,
        &server,
        &[
            "rename",
            &private_created.board_id,
            "--title",
            "unauthorized rename",
        ],
    )
    .await
    .expect_err("an unrelated ordinary user must not rename a private board");
    assert_eq!(denied_rename.code(), ErrorCode::PermissionDenied);
    let denied_delete = execute_live_board(
        &owner_app,
        &server,
        &["delete", &private_created.board_id, "--yes"],
    )
    .await
    .expect_err("a non-site-admin board owner must not delete an unrelated board");
    assert_eq!(denied_delete.code(), ErrorCode::PermissionDenied);
    let CommandSuccess::BoardShown(still_private) =
        execute_live_board(&app, &server, &["get", &private_created.board_id])
            .await
            .expect("denied ordinary-user mutations must leave the private board unchanged")
    else {
        panic!("expected board-detail output")
    };
    assert_eq!(still_private.title, private_title);

    let owner_title = format!("Owner lifecycle {nonce}");
    let CommandSuccess::BoardCreated(owner_created) =
        execute_live_board(&owner_app, &server, &["create", "--title", &owner_title])
            .await
            .expect("an ordinary logged-in user must be able to create a board")
    else {
        panic!("expected create-board output")
    };
    let owner_renamed_title = format!("Owner renamed lifecycle {nonce}");
    let padded_owner_renamed_title = format!("  {owner_renamed_title}  ");
    let CommandSuccess::BoardRenamed(owner_renamed) = execute_live_board(
        &owner_app,
        &server,
        &[
            "rename",
            &owner_created.board_id,
            "--title",
            &padded_owner_renamed_title,
        ],
    )
    .await
    .expect("an ordinary board owner must be able to rename their board") else {
        panic!("expected rename-board output")
    };
    assert_eq!(owner_renamed.title, owner_renamed_title);
    let CommandSuccess::BoardShown(owner_renamed_board) =
        execute_live_board(&owner_app, &server, &["get", &owner_created.board_id])
            .await
            .expect("the ordinary user's rename must be persisted")
    else {
        panic!("expected board-detail output")
    };
    assert_eq!(owner_renamed_board.title, owner_renamed_title);
    let CommandSuccess::BoardDeleted(owner_deleted) = execute_live_board(
        &owner_app,
        &server,
        &["delete", &owner_created.board_id, "--yes"],
    )
    .await
    .expect("an ordinary board admin must be able to delete their own board") else {
        panic!("expected delete-board output")
    };
    assert_eq!(owner_deleted.board_id, owner_created.board_id);
    wait_for_live_board_absent(
        &reqwest::Client::new(),
        &canonical_server,
        &admin_token,
        &owner_created.board_id,
    )
    .await;

    let renamed_title = format!("Renamed lifecycle {nonce}");
    let CommandSuccess::BoardRenamed(renamed) = execute_live_board(
        &app,
        &server,
        &[
            "rename",
            &private_created.board_id,
            "--title",
            &renamed_title,
        ],
    )
    .await
    .expect("board rename must succeed") else {
        panic!("expected rename-board output")
    };
    assert_eq!(renamed.board_id, private_created.board_id);
    assert_eq!(renamed.title, renamed_title);
    let CommandSuccess::BoardShown(renamed_board) =
        execute_live_board(&app, &server, &["get", &private_created.board_id])
            .await
            .expect("the site-admin rename must be persisted")
    else {
        panic!("expected board-detail output")
    };
    assert_eq!(renamed_board.title, renamed_title);

    for board_id in [
        &private_created.board_id,
        &public_created.board_id,
        &copied_board_id,
    ] {
        let CommandSuccess::BoardDeleted(deleted) =
            execute_live_board(&app, &server, &["delete", board_id, "--yes"])
                .await
                .expect("board deletion must succeed")
        else {
            panic!("expected delete-board output")
        };
        assert_eq!(&deleted.board_id, board_id);
        assert!(deleted.deleted);
        wait_for_live_board_absent(
            &reqwest::Client::new(),
            &canonical_server,
            &admin_token,
            board_id,
        )
        .await;
    }

    let CommandSuccess::BoardCount(after_delete) = execute_live_board(&app, &server, &["count"])
        .await
        .expect("the post-delete board count must succeed")
    else {
        panic!("expected board-count output")
    };
    assert_eq!(after_delete, baseline);

    let missing_board = execute_live_board(&app, &server, &["get", &private_created.board_id])
        .await
        .expect_err("a deleted board must be reported as missing");
    assert_eq!(missing_board.code(), ErrorCode::NotFound);
    assert_eq!(missing_board.details().http_status, Some(200));
    assert_eq!(missing_board.details().wekan_status_code, Some(404));
    assert_eq!(
        missing_board.details().server_reason.as_deref(),
        Some("Board not found")
    );
    assert_eq!(missing_board.details().outcome_unknown, None);
}
