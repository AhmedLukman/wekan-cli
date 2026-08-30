use super::support::*;
#[tokio::test]
#[ignore = "destructive: requires an isolated Wekan v11.06 stack and WEKAN_SWIMLANE_E2E_URL"]
async fn complete_swimlane_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_SWIMLANE_E2E_URL")
        .expect("set WEKAN_SWIMLANE_E2E_URL to an isolated Wekan v11.06 server URL");
    let canonical_server = ServerUrl::parse(&server)
        .expect("the live-test Wekan URL must be valid")
        .as_str()
        .to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock must be after the Unix epoch")
        .as_millis();
    let username = format!("wekan_cli_swimlane_admin_{nonce}");
    let email = format!("{username}@example.test");
    let password = format!("Wekan-swimlane-e2e-{nonce}!");

    let directory = TestDirectory::new("complete-swimlane-lifecycle");
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
    let CommandSuccess::Registration(_) = app
        .execute(registration.command)
        .await
        .expect("the swimlane lifecycle account must register")
    else {
        panic!("expected registration output")
    };
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .expect("registration must store the swimlane-test credential")
        .3
        .clone();

    let board_title = format!("Swimlane lifecycle {nonce}");
    let CommandSuccess::BoardCreated(board) =
        execute_live_board(&app, &server, &["create", "--title", &board_title])
            .await
            .expect("the swimlane lifecycle board must be created")
    else {
        panic!("expected create-board output")
    };

    let member_username = format!("wekan_cli_swimlane_member_{nonce}");
    let member_email = format!("{member_username}@example.test");
    let member_password = format!("Wekan-swimlane-member-{nonce}!");
    let member_directory = TestDirectory::new("complete-swimlane-member");
    let member_app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(member_password)),
        FileProfileStore::at(member_directory.path().to_owned()),
    );
    let member_registration = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "register",
        "--username",
        &member_username,
        "--email",
        &member_email,
        "--password-stdin",
    ])
    .unwrap();
    let CommandSuccess::Registration(_) = member_app
        .execute(member_registration.command)
        .await
        .expect("the ordinary swimlane-test account must register")
    else {
        panic!("expected member registration output")
    };
    let member_user_id = member_app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .expect("member registration must store its credential")
        .2
        .clone();
    let membership = live_api_json(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        reqwest::Method::POST,
        &format!(
            "api/boards/{}/members/{}/add",
            board.board_id, member_user_id
        ),
        Some(serde_json::json!({
            "action": "add",
            "role": "readonly"
        })),
    )
    .await;
    assert!(
        membership.as_array().is_some_and(|boards| {
            boards
                .iter()
                .any(|summary| summary["_id"].as_str() == Some(board.board_id.as_str()))
        }),
        "the board-admin membership request must return the affected board"
    );
    wait_for_live_board_membership(&member_app, &server, &board.board_id).await;
    let CommandSuccess::BoardShown(member_board) =
        execute_live_board(&member_app, &server, &["get", &board.board_id])
            .await
            .expect("the ordinary member must be able to read the board")
    else {
        panic!("expected member board-detail output")
    };
    let member = member_board
        .members
        .iter()
        .find(|member| member.user_id == member_user_id)
        .expect("the read-only member must be present on the board");
    assert!(member.is_active);
    assert!(!member.is_admin);
    assert_eq!(member.is_read_only, Some(true));

    let initial_title = format!("Delivery {nonce}");
    let CommandSuccess::SwimlaneCreated(created) = execute_live_swimlane(
        &app,
        &server,
        &[
            "create",
            &board.board_id,
            "--title",
            &initial_title,
            "--sort",
            "7.25",
        ],
    )
    .await
    .expect("swimlane creation with an explicit sort must succeed") else {
        panic!("expected create-swimlane output")
    };
    assert_eq!(created.board_id, board.board_id);
    assert!(!created.swimlane_id.is_empty());

    let CommandSuccess::SwimlaneCollection(collection) =
        execute_live_swimlane(&app, &server, &["list", &board.board_id])
            .await
            .expect("the created swimlane must be visible in the board collection")
    else {
        panic!("expected swimlane collection output")
    };
    assert!(collection.swimlanes.iter().any(|swimlane| {
        swimlane.swimlane_id == created.swimlane_id && swimlane.title == initial_title
    }));

    let CommandSuccess::SwimlaneShown(initial) = execute_live_swimlane(
        &app,
        &server,
        &["get", &board.board_id, &created.swimlane_id],
    )
    .await
    .expect("the created swimlane must be readable") else {
        panic!("expected swimlane detail output")
    };
    assert_eq!(initial.swimlane_id, created.swimlane_id);
    assert_eq!(initial.board_id, board.board_id);
    assert_eq!(initial.title, initial_title);
    assert!(!initial.archived);
    assert_eq!(initial.sort.and_then(|sort| sort.as_f64()), Some(7.25));
    assert_eq!(initial.swimlane_type, "swimlane");
    assert_eq!(initial.height.and_then(|height| height.as_i64()), Some(-1));

    let appended_title = format!("Appended {nonce}");
    let CommandSuccess::SwimlaneCreated(appended) = execute_live_swimlane(
        &app,
        &server,
        &["create", &board.board_id, "--title", &appended_title],
    )
    .await
    .expect("swimlane creation without an explicit sort must succeed") else {
        panic!("expected appended create-swimlane output")
    };
    let CommandSuccess::SwimlaneShown(appended_detail) = execute_live_swimlane(
        &app,
        &server,
        &["get", &board.board_id, &appended.swimlane_id],
    )
    .await
    .expect("the appended swimlane must be readable") else {
        panic!("expected appended swimlane detail")
    };
    assert_eq!(appended_detail.title, appended_title);
    assert_eq!(
        appended_detail.sort.and_then(|sort| sort.as_f64()),
        Some(8.25),
        "omitting --sort must append after the non-contiguous maximum sort"
    );

    let CommandSuccess::SwimlaneCollection(member_collection) =
        execute_live_swimlane(&member_app, &server, &["list", &board.board_id])
            .await
            .expect("the ordinary member must be able to list swimlanes")
    else {
        panic!("expected member swimlane collection output")
    };
    assert!(
        member_collection
            .swimlanes
            .iter()
            .any(|swimlane| swimlane.swimlane_id == created.swimlane_id)
    );
    let CommandSuccess::SwimlaneShown(member_swimlane) = execute_live_swimlane(
        &member_app,
        &server,
        &["get", &board.board_id, &created.swimlane_id],
    )
    .await
    .expect("the ordinary member must be able to get a swimlane") else {
        panic!("expected member swimlane detail output")
    };
    assert_eq!(member_swimlane.swimlane_id, created.swimlane_id);

    let denied_create = execute_live_swimlane(
        &member_app,
        &server,
        &[
            "create",
            &board.board_id,
            "--title",
            "Read-only create must fail",
        ],
    )
    .await
    .expect_err("a read-only member must not create a swimlane");
    assert_eq!(denied_create.code(), ErrorCode::PermissionDenied);
    let denied_update = execute_live_swimlane(
        &member_app,
        &server,
        &[
            "update",
            &board.board_id,
            &created.swimlane_id,
            "--title",
            "Read-only update must fail",
        ],
    )
    .await
    .expect_err("a read-only member must not update a swimlane");
    assert_eq!(denied_update.code(), ErrorCode::PermissionDenied);
    let denied_delete = execute_live_swimlane(
        &member_app,
        &server,
        &["delete", &board.board_id, &created.swimlane_id, "--yes"],
    )
    .await
    .expect_err("a read-only member must not delete a swimlane");
    assert_eq!(denied_delete.code(), ErrorCode::PermissionDenied);

    let updated_title = format!("Operations {nonce}");
    let CommandSuccess::SwimlaneUpdated(updated) = execute_live_swimlane(
        &app,
        &server,
        &[
            "update",
            &board.board_id,
            &created.swimlane_id,
            "--title",
            &updated_title,
        ],
    )
    .await
    .expect("swimlane title update must succeed") else {
        panic!("expected update-swimlane output")
    };
    assert_eq!(
        updated.updated_fields,
        vec![wekan_cli::command_result::SwimlaneUpdatedField::Title]
    );

    let CommandSuccess::SwimlaneShown(after_update) = execute_live_swimlane(
        &app,
        &server,
        &["get", &board.board_id, &created.swimlane_id],
    )
    .await
    .expect("the updated swimlane must remain readable") else {
        panic!("expected swimlane detail output")
    };
    assert_eq!(after_update.title, updated_title);

    let list_title = format!("Cascade list {nonce}");
    let CommandSuccess::ListCreated(list) = execute_live_list(
        &app,
        &server,
        &[
            "create",
            &board.board_id,
            "--title",
            &list_title,
            "--swimlane-id",
            &created.swimlane_id,
        ],
    )
    .await
    .expect("the cascade list must be created in the target swimlane") else {
        panic!("expected create-list output")
    };
    let card = live_api_json(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        reqwest::Method::POST,
        &format!("api/boards/{}/lists/{}/cards", board.board_id, list.list_id),
        Some(serde_json::json!({
            "title": format!("Cascade card {nonce}"),
            "swimlaneId": created.swimlane_id
        })),
    )
    .await;
    let card_id = required_id(&card, "swimlane cascade card");

    let CommandSuccess::SwimlaneDeleted(deleted) = execute_live_swimlane(
        &app,
        &server,
        &["delete", &board.board_id, &created.swimlane_id, "--yes"],
    )
    .await
    .expect("hard swimlane deletion must succeed") else {
        panic!("expected delete-swimlane output")
    };
    assert_eq!(deleted.board_id, board.board_id);
    assert_eq!(deleted.swimlane_id, created.swimlane_id);
    assert!(deleted.deleted);
    assert_eq!(deleted.delete_mode, SwimlaneDeleteMode::Hard);

    wait_for_live_swimlane_absent(&app, &server, &board.board_id, &created.swimlane_id).await;
    wait_for_live_swimlane_absent_from_collection(
        &app,
        &server,
        &board.board_id,
        &created.swimlane_id,
    )
    .await;

    let CommandSuccess::SwimlaneDeleted(repeated) = execute_live_swimlane(
        &app,
        &server,
        &["delete", &board.board_id, &created.swimlane_id, "--yes"],
    )
    .await
    .expect("repeated swimlane deletion must remain idempotent") else {
        panic!("expected repeated delete-swimlane output")
    };
    assert_eq!(repeated.swimlane_id, created.swimlane_id);
    assert!(repeated.deleted);

    wait_for_live_list_absent(&app, &server, &board.board_id, &list.list_id).await;
    wait_for_live_card_absent(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        &board.board_id,
        &list.list_id,
        &card_id,
    )
    .await;

    let first_preserved_title = format!("Preserved first {nonce}");
    let CommandSuccess::ListCreated(first_preserved_list) = execute_live_list(
        &app,
        &server,
        &[
            "create",
            &board.board_id,
            "--title",
            &first_preserved_title,
            "--swimlane-id",
            &appended.swimlane_id,
        ],
    )
    .await
    .expect("the first preserved-branch list must be created") else {
        panic!("expected first preserved-branch list output")
    };
    let second_preserved_title = format!("Preserved second {nonce}");
    let CommandSuccess::ListCreated(second_preserved_list) = execute_live_list(
        &app,
        &server,
        &[
            "create",
            &board.board_id,
            "--title",
            &second_preserved_title,
            "--swimlane-id",
            &appended.swimlane_id,
        ],
    )
    .await
    .expect("the second preserved-branch list must be created") else {
        panic!("expected second preserved-branch list output")
    };
    let first_preserved_card = live_api_json(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        reqwest::Method::POST,
        &format!(
            "api/boards/{}/lists/{}/cards",
            board.board_id, first_preserved_list.list_id
        ),
        Some(serde_json::json!({
            "title": format!("First preserved-branch card {nonce}"),
            "swimlaneId": appended.swimlane_id
        })),
    )
    .await;
    let first_preserved_card_id = required_id(&first_preserved_card, "first preserved-branch card");
    let second_preserved_card = live_api_json(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        reqwest::Method::POST,
        &format!(
            "api/boards/{}/lists/{}/cards",
            board.board_id, second_preserved_list.list_id
        ),
        Some(serde_json::json!({
            "title": format!("Second preserved-branch card {nonce}"),
            "swimlaneId": appended.swimlane_id
        })),
    )
    .await;
    let second_preserved_card_id =
        required_id(&second_preserved_card, "second preserved-branch card");

    let CommandSuccess::SwimlaneDeleted(preserved_branch_delete) = execute_live_swimlane(
        &app,
        &server,
        &["delete", &board.board_id, &appended.swimlane_id, "--yes"],
    )
    .await
    .expect("deleting a swimlane with two matching lists must succeed") else {
        panic!("expected preserved-branch delete-swimlane output")
    };
    assert_eq!(preserved_branch_delete.swimlane_id, appended.swimlane_id);
    assert!(preserved_branch_delete.deleted);
    wait_for_live_swimlane_absent(&app, &server, &board.board_id, &appended.swimlane_id).await;
    wait_for_live_swimlane_absent_from_collection(
        &app,
        &server,
        &board.board_id,
        &appended.swimlane_id,
    )
    .await;
    wait_for_live_card_absent(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        &board.board_id,
        &first_preserved_list.list_id,
        &first_preserved_card_id,
    )
    .await;
    wait_for_live_card_absent(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        &board.board_id,
        &second_preserved_list.list_id,
        &second_preserved_card_id,
    )
    .await;
    let CommandSuccess::ListShown(first_preserved) = execute_live_list(
        &app,
        &server,
        &["get", &board.board_id, &first_preserved_list.list_id],
    )
    .await
    .expect("the first list must survive the two-list swimlane deletion branch") else {
        panic!("expected first preserved list detail")
    };
    let CommandSuccess::ListShown(second_preserved) = execute_live_list(
        &app,
        &server,
        &["get", &board.board_id, &second_preserved_list.list_id],
    )
    .await
    .expect("the second list must survive the two-list swimlane deletion branch") else {
        panic!("expected second preserved list detail")
    };
    assert_eq!(first_preserved.title, first_preserved_title);
    assert_eq!(second_preserved.title, second_preserved_title);
}
