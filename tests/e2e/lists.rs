use super::support::*;
#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v11.06 stack and WEKAN_LIST_E2E_URL"]
async fn complete_list_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_LIST_E2E_URL")
        .expect("set WEKAN_LIST_E2E_URL to a fresh isolated Wekan v11.06 server URL");
    let canonical_server = ServerUrl::parse(&server)
        .expect("the live-test Wekan URL must be valid")
        .as_str()
        .to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock must be after the Unix epoch")
        .as_millis();
    let username = format!("wekan_cli_list_admin_{nonce}");
    let email = format!("{username}@example.test");
    let password = format!("Wekan-list-e2e-{nonce}!");

    let directory = TestDirectory::new("complete-list-lifecycle");
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
        .expect("the first account on the fresh list stack must register as administrator")
    else {
        panic!("expected registration output")
    };
    let token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .expect("registration must store the administrator credential")
        .3
        .clone();

    let board_title = format!("List lifecycle {nonce}");
    let CommandSuccess::BoardCreated(board) =
        execute_live_board(&app, &server, &["create", "--title", &board_title])
            .await
            .expect("the list lifecycle board must be created")
    else {
        panic!("expected create-board output")
    };

    let initial_title = format!("Todo {nonce}");
    let CommandSuccess::ListCreated(created) = execute_live_list(
        &app,
        &server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--title",
            &initial_title,
            "--swimlane-id",
            &board.default_swimlane_id,
        ],
    )
    .await
    .expect("list creation must succeed") else {
        panic!("expected create-list output")
    };
    assert_eq!(created.board_id, board.board_id);
    assert!(!created.list_id.is_empty());

    let CommandSuccess::ListCollection(collection) =
        execute_live_list(&app, &server, &["list", "--board", &board.board_id])
            .await
            .expect("the created list must be visible in the board collection")
    else {
        panic!("expected list collection output")
    };
    assert!(
        collection
            .lists
            .iter()
            .any(|list| { list.list_id == created.list_id && list.title == initial_title })
    );

    let CommandSuccess::ListShown(initial) = execute_live_list(
        &app,
        &server,
        &["get", "--board", &board.board_id, &created.list_id],
    )
    .await
    .expect("the created list must be readable") else {
        panic!("expected list detail output")
    };
    assert_eq!(initial.list_id, created.list_id);
    assert_eq!(initial.board_id, board.board_id);
    assert_eq!(
        initial.swimlane_id.as_deref(),
        Some(board.default_swimlane_id.as_str())
    );
    assert_eq!(initial.title, initial_title);
    assert!(!initial.archived);
    assert_eq!(initial.list_type, "list");

    let permissive_create = live_api_json(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        reqwest::Method::POST,
        &format!("api/boards/{}/lists", board.board_id),
        Some(serde_json::json!({
            "title": format!("Extra-field list {nonce}"),
            "swimlaneId": board.default_swimlane_id,
            "futureIgnored": true
        })),
    )
    .await;
    assert!(!required_id(&permissive_create, "extra-field list").is_empty());

    let permissive_update = live_api_json(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        reqwest::Method::PUT,
        &format!("api/boards/{}/lists/{}", board.board_id, created.list_id),
        Some(serde_json::json!({
            "title": initial_title,
            "futureIgnored": true
        })),
    )
    .await;
    assert_eq!(
        required_id(&permissive_update, "extra-field list update"),
        created.list_id
    );

    let card = live_api_json(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        reqwest::Method::POST,
        &format!(
            "api/boards/{}/lists/{}/cards",
            board.board_id, created.list_id
        ),
        Some(serde_json::json!({
            "title": format!("Cascade card {nonce}"),
            "swimlaneId": board.default_swimlane_id
        })),
    )
    .await;
    let card_id = required_id(&card, "card");

    let updated_title = format!("Doing {nonce}");
    let CommandSuccess::ListUpdated(updated) = execute_live_list(
        &app,
        &server,
        &[
            "update",
            "--board",
            &board.board_id,
            &created.list_id,
            "--title",
            &updated_title,
            "--color",
            "#12aBcF",
            "--starred",
            "true",
            "--wip-limit",
            "3.5",
            "--wip-enabled",
            "true",
            "--wip-soft",
            "false",
        ],
    )
    .await
    .expect("the multi-field list update must succeed") else {
        panic!("expected update-list output")
    };
    assert_eq!(
        updated.updated_fields,
        vec![
            wekan_cli::command_result::ListUpdatedField::Title,
            wekan_cli::command_result::ListUpdatedField::Color,
            wekan_cli::command_result::ListUpdatedField::Starred,
            wekan_cli::command_result::ListUpdatedField::WipLimit,
        ]
    );

    let CommandSuccess::ListShown(after_update) = execute_live_list(
        &app,
        &server,
        &["get", "--board", &board.board_id, &created.list_id],
    )
    .await
    .expect("the updated list must remain readable") else {
        panic!("expected list detail output")
    };
    assert_eq!(after_update.title, updated_title);
    assert_eq!(after_update.color.as_deref(), Some("#12aBcF"));
    assert_eq!(after_update.starred, Some(true));
    let wip = after_update
        .wip_limit
        .expect("the complete WIP update must be returned");
    assert_eq!(wip.value.as_f64(), Some(3.5));
    assert!(wip.enabled);
    assert!(!wip.soft);

    let submitted_long_title = "x".repeat(1001);
    let CommandSuccess::ListUpdated(truncated_title_update) = execute_live_list(
        &app,
        &server,
        &[
            "update",
            "--board",
            &board.board_id,
            &created.list_id,
            "--title",
            &submitted_long_title,
        ],
    )
    .await
    .expect("the CLI must submit a list title that Wekan truncates") else {
        panic!("expected update-list output")
    };
    assert_eq!(
        truncated_title_update.updated_fields,
        [wekan_cli::command_result::ListUpdatedField::Title]
    );
    let CommandSuccess::ListShown(truncated_title_list) = execute_live_list(
        &app,
        &server,
        &["get", "--board", &board.board_id, &created.list_id],
    )
    .await
    .expect("the list with Wekan's truncated title must remain readable") else {
        panic!("expected list detail output")
    };
    assert_eq!(truncated_title_list.title, "x".repeat(1000));

    let permissive_wip_update = live_api_json(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        reqwest::Method::PUT,
        &format!("api/boards/{}/lists/{}", board.board_id, created.list_id),
        Some(serde_json::json!({
            "wipLimit": {
                "value": 4,
                "enabled": true,
                "soft": false,
                "futureIgnored": true
            }
        })),
    )
    .await;
    assert_eq!(
        required_id(&permissive_wip_update, "permissive WIP update"),
        created.list_id
    );

    let CommandSuccess::ListShown(sanitized_wip) = execute_live_list(
        &app,
        &server,
        &["get", "--board", &board.board_id, &created.list_id],
    )
    .await
    .expect("Wekan must strip unsupported WIP fields before returning the list") else {
        panic!("expected list detail output")
    };
    assert_eq!(
        sanitized_wip
            .wip_limit
            .expect("the normalized WIP limit must be returned")
            .value
            .as_i64(),
        Some(4)
    );

    let CommandSuccess::ListDeleted(deleted) = execute_live_list(
        &app,
        &server,
        &[
            "delete",
            "--board",
            &board.board_id,
            &created.list_id,
            "--yes",
        ],
    )
    .await
    .expect("soft deletion must succeed") else {
        panic!("expected delete-list output")
    };
    assert_eq!(deleted.board_id, board.board_id);
    assert_eq!(deleted.list_id, created.list_id);
    assert!(deleted.deleted);
    assert_eq!(deleted.delete_mode, ListDeleteMode::Soft);

    let CommandSuccess::ListShown(soft_deleted) = execute_live_list(
        &app,
        &server,
        &["get", "--board", &board.board_id, &created.list_id],
    )
    .await
    .expect("Wekan must retain the soft-deleted list document") else {
        panic!("expected list detail output")
    };
    assert!(soft_deleted.deleted_at.is_some());
    let delete_batch_id = soft_deleted
        .delete_batch_id
        .expect("the soft-deleted list must identify its deletion batch");

    let deleted_card = live_api_json(
        &reqwest::Client::new(),
        &canonical_server,
        &token,
        reqwest::Method::GET,
        &format!(
            "api/boards/{}/lists/{}/cards/{card_id}",
            board.board_id, created.list_id
        ),
        None,
    )
    .await;
    assert!(
        deleted_card
            .get("deletedAt")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "the list delete must soft-delete its live card: {deleted_card}"
    );
    assert_eq!(
        deleted_card
            .get("deleteBatchId")
            .and_then(serde_json::Value::as_str),
        Some(delete_batch_id.as_str())
    );
}
