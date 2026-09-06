use super::support::*;
#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v11.06 stack and WEKAN_CARD_E2E_URL"]
async fn complete_card_lifecycle_matches_wekan_v11_06() {
    let server = env::var("WEKAN_CARD_E2E_URL")
        .expect("set WEKAN_CARD_E2E_URL to a fresh isolated Wekan v11.06 server URL");
    let canonical_server = ServerUrl::parse(&server)
        .expect("the live-test Wekan URL must be valid")
        .as_str()
        .to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock must be after the Unix epoch")
        .as_millis();
    let username = format!("wekan_cli_card_admin_{nonce}");
    let email = format!("{username}@example.test");
    let password = format!("Wekan-card-e2e-{nonce}!");

    let directory = TestDirectory::new("complete-card-lifecycle");
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
        .expect("the first account on the fresh card stack must register as administrator")
    else {
        panic!("expected registration output")
    };
    let admin_id = registration.user_id;
    let token = app
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
        &["create", "--title", &format!("Card lifecycle {nonce}")],
    )
    .await
    .expect("the card lifecycle board must be created") else {
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
    .expect("the card lifecycle list must be created") else {
        panic!("expected create-list output")
    };
    let CommandSuccess::BoardShown(board_detail) =
        execute_live_board(&app, &server, &["get", &board.board_id])
            .await
            .expect("the card lifecycle board must be readable")
    else {
        panic!("expected board detail output")
    };
    let label_id = if let Some(label) = board_detail.labels.first() {
        label.label_id.clone()
    } else {
        live_api_json(
            &http,
            &canonical_server,
            &token,
            reqwest::Method::PUT,
            &format!("api/boards/{}/labels", board.board_id),
            Some(serde_json::json!({
                "label": {"name": "Lifecycle", "color": "red"}
            })),
        )
        .await
        .as_str()
        .expect("board-label creation must return its ID")
        .to_owned()
    };

    let CommandSuccess::CardCreated(parent) = execute_live_card(
        &app,
        &server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--title",
            "Parent",
            "--swimlane-id",
            &board.default_swimlane_id,
        ],
    )
    .await
    .expect("the parent card must be created") else {
        panic!("expected card creation output")
    };

    let (_normal_directory, normal_app, _) = register_live_board_role(
        &server,
        &canonical_server,
        &token,
        &board.board_id,
        nonce,
        "normal",
    )
    .await;
    let (_comment_directory, comment_app, _) = register_live_board_role(
        &server,
        &canonical_server,
        &token,
        &board.board_id,
        nonce,
        "commentonly",
    )
    .await;
    let (_worker_directory, worker_app, _) = register_live_board_role(
        &server,
        &canonical_server,
        &token,
        &board.board_id,
        nonce,
        "worker",
    )
    .await;
    let (_read_only_directory, read_only_app, _) = register_live_board_role(
        &server,
        &canonical_server,
        &token,
        &board.board_id,
        nonce,
        "readonly",
    )
    .await;

    for (role, role_app) in [("normal", &normal_app), ("read-only", &read_only_app)] {
        let CommandSuccess::CardCollection(cards) = execute_live_card(
            role_app,
            &server,
            &["list", "--board", &board.board_id, "--list", &list.list_id],
        )
        .await
        .unwrap_or_else(|error| panic!("{role} member must be able to list cards: {error:?}")) else {
            panic!("expected {role} card collection output")
        };
        assert!(
            cards
                .cards
                .iter()
                .any(|card| card.card_id == parent.card_id),
            "{role} member must see the permission-probe card"
        );
        let CommandSuccess::CardShown(card) = execute_live_card(
            role_app,
            &server,
            &[
                "get",
                "--board",
                &board.board_id,
                "--list",
                &list.list_id,
                &parent.card_id,
            ],
        )
        .await
        .unwrap_or_else(|error| panic!("{role} member must be able to get a card: {error:?}")) else {
            panic!("expected {role} card detail output")
        };
        assert_eq!(card.card_id, parent.card_id);
    }

    for (role, role_app) in [("comment-only", &comment_app), ("worker", &worker_app)] {
        let list_error = execute_live_card(
            role_app,
            &server,
            &["list", "--board", &board.board_id, "--list", &list.list_id],
        )
        .await
        .expect_err("Wekan must deny card listing for this board role");
        assert_eq!(
            list_error.code(),
            ErrorCode::PermissionDenied,
            "{role} list"
        );

        let get_error = execute_live_card(
            role_app,
            &server,
            &[
                "get",
                "--board",
                &board.board_id,
                "--list",
                &list.list_id,
                &parent.card_id,
            ],
        )
        .await
        .expect_err("Wekan must deny card retrieval for this board role");
        assert_eq!(get_error.code(), ErrorCode::PermissionDenied, "{role} get");
    }

    let CommandSuccess::CardCreated(normal_card) = execute_live_card(
        &normal_app,
        &server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--title",
            "Normal member permission probe",
            "--swimlane-id",
            &board.default_swimlane_id,
        ],
    )
    .await
    .expect("a normal member must be able to create a card") else {
        panic!("expected normal-member card creation output")
    };
    execute_live_card(
        &normal_app,
        &server,
        &[
            "update",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &normal_card.card_id,
            "--title",
            "Normal member updated permission probe",
        ],
    )
    .await
    .expect("a normal member must be able to update a card");
    execute_live_card(
        &normal_app,
        &server,
        &[
            "delete",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &normal_card.card_id,
            "--yes",
        ],
    )
    .await
    .expect("a normal member must be able to delete a card");

    let mut permission_cleanup_ids = Vec::new();
    for (role, role_app) in [("comment-only", &comment_app), ("worker", &worker_app)] {
        let CommandSuccess::CardCreated(role_card) = execute_live_card(
            role_app,
            &server,
            &[
                "create",
                "--board",
                &board.board_id,
                "--list",
                &list.list_id,
                "--title",
                &format!("{role} create-permission defect probe"),
                "--swimlane-id",
                &board.default_swimlane_id,
            ],
        )
        .await
        .unwrap_or_else(|error| panic!("v11.06 permits {role} members to create cards: {error:?}")) else {
            panic!("expected {role} card creation output")
        };
        let denied_update = match execute_live_card(
            role_app,
            &server,
            &[
                "update",
                "--board",
                &board.board_id,
                "--list",
                &list.list_id,
                &role_card.card_id,
                "--title",
                "This update must be denied",
            ],
        )
        .await
        {
            Err(error) => error,
            Ok(success) => panic!("{role} update unexpectedly succeeded: {success:?}"),
        };
        assert_eq!(denied_update.code(), ErrorCode::PermissionDenied);
        let denied_delete = match execute_live_card(
            role_app,
            &server,
            &[
                "delete",
                "--board",
                &board.board_id,
                "--list",
                &list.list_id,
                &role_card.card_id,
                "--yes",
            ],
        )
        .await
        {
            Err(error) => error,
            Ok(success) => panic!("{role} delete unexpectedly succeeded: {success:?}"),
        };
        assert_eq!(denied_delete.code(), ErrorCode::PermissionDenied);
        permission_cleanup_ids.push(role_card.card_id);
    }

    for operation in ["create", "update", "delete"] {
        let arguments = match operation {
            "create" => vec![
                "create",
                "--board",
                &board.board_id,
                "--list",
                &list.list_id,
                "--title",
                "Read-only create must fail",
                "--swimlane-id",
                &board.default_swimlane_id,
            ],
            "update" => vec![
                "update",
                "--board",
                &board.board_id,
                "--list",
                &list.list_id,
                &parent.card_id,
                "--title",
                "Read-only update must fail",
            ],
            "delete" => vec![
                "delete",
                "--board",
                &board.board_id,
                "--list",
                &list.list_id,
                &parent.card_id,
                "--yes",
            ],
            _ => unreachable!(),
        };
        let error = match execute_live_card(&read_only_app, &server, &arguments).await {
            Err(error) => error,
            Ok(success) => {
                panic!("read-only {operation} unexpectedly succeeded: {success:?}")
            }
        };
        assert_eq!(error.code(), ErrorCode::PermissionDenied);
    }

    let initial_title = format!("Card {nonce}");
    let CommandSuccess::CardCreated(created) = execute_live_card(
        &app,
        &server,
        &[
            "create",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            "--title",
            &initial_title,
            "--swimlane-id",
            &board.default_swimlane_id,
            "--description",
            "Initial details",
            "--member",
            &admin_id,
            "--assignee",
            &admin_id,
            "--received-at",
            "2030-01-01T00:00:00Z",
            "--start-at",
            "2030-01-02T00:00:00Z",
            "--due-at",
            "2030-01-03T00:00:00Z",
            "--end-at",
            "2030-01-04T00:00:00Z",
        ],
    )
    .await
    .expect("card creation with every supported optional field must succeed") else {
        panic!("expected card creation output")
    };
    assert_eq!(created.board_id, board.board_id);
    assert_eq!(created.list_id, list.list_id);
    assert!(!created.card_id.is_empty());

    let CommandSuccess::CardCollection(collection) = execute_live_card(
        &app,
        &server,
        &["list", "--board", &board.board_id, "--list", &list.list_id],
    )
    .await
    .expect("the created card must be visible in its list") else {
        panic!("expected card collection output")
    };
    assert!(collection.cards.iter().any(|card| {
        card.card_id == created.card_id && card.title.as_deref() == Some(initial_title.as_str())
    }));

    let CommandSuccess::CardShown(initial) = execute_live_card(
        &app,
        &server,
        &[
            "get",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &created.card_id,
        ],
    )
    .await
    .expect("the created card must be readable through the strict decoder") else {
        panic!("expected card detail output")
    };
    assert_eq!(initial.title.as_deref(), Some(initial_title.as_str()));
    assert_eq!(initial.description.as_deref(), Some("Initial details"));
    assert_eq!(initial.members, [admin_id.clone()]);
    assert_eq!(initial.assignees, [admin_id.clone()]);
    assert_eq!(
        initial.received_at.as_deref(),
        Some("2030-01-01T00:00:00.000Z")
    );
    assert_eq!(
        initial.start_at.as_deref(),
        Some("2030-01-02T00:00:00.000Z")
    );
    assert_eq!(initial.due_at.as_deref(), Some("2030-01-03T00:00:00.000Z"));
    assert_eq!(initial.end_at.as_deref(), Some("2030-01-04T00:00:00.000Z"));
    assert_eq!(initial.card_type, "cardType-card");

    let updated_title = format!("Updated card {nonce}");
    let CommandSuccess::CardUpdated(updated) = execute_live_card(
        &app,
        &server,
        &[
            "update",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &created.card_id,
            "--title",
            &updated_title,
            "--sort",
            "73.25",
            "--parent-id",
            &parent.card_id,
            "--description",
            "Updated details",
            "--color",
            "#12aBcF",
            "--label",
            &label_id,
            "--requested-by",
            "Requester",
            "--assigned-by",
            "Dispatcher",
            "--received-at",
            "2031-01-01T00:00:00Z",
            "--start-at",
            "2031-01-02T00:00:00Z",
            "--due-at",
            "2031-01-03T00:00:00Z",
            "--end-at",
            "2031-01-04T00:00:00Z",
            "--spent-time",
            "2.5",
            "--is-over-time",
            "true",
            "--member",
            &admin_id,
            "--assignee",
            &admin_id,
            "--due-complete",
            "true",
        ],
    )
    .await
    .expect("the complete supported card update must succeed") else {
        panic!("expected card update output")
    };
    assert_eq!(
        updated.submitted_fields,
        vec![
            wekan_cli::command_result::CardSubmittedField::Title,
            wekan_cli::command_result::CardSubmittedField::Sort,
            wekan_cli::command_result::CardSubmittedField::ParentId,
            wekan_cli::command_result::CardSubmittedField::Description,
            wekan_cli::command_result::CardSubmittedField::Color,
            wekan_cli::command_result::CardSubmittedField::LabelIds,
            wekan_cli::command_result::CardSubmittedField::RequestedBy,
            wekan_cli::command_result::CardSubmittedField::AssignedBy,
            wekan_cli::command_result::CardSubmittedField::ReceivedAt,
            wekan_cli::command_result::CardSubmittedField::StartAt,
            wekan_cli::command_result::CardSubmittedField::DueAt,
            wekan_cli::command_result::CardSubmittedField::EndAt,
            wekan_cli::command_result::CardSubmittedField::SpentTime,
            wekan_cli::command_result::CardSubmittedField::IsOverTime,
            wekan_cli::command_result::CardSubmittedField::Members,
            wekan_cli::command_result::CardSubmittedField::Assignees,
            wekan_cli::command_result::CardSubmittedField::DueComplete,
        ]
    );
    let CommandSuccess::CardShown(after_update) = execute_live_card(
        &app,
        &server,
        &[
            "get",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &created.card_id,
        ],
    )
    .await
    .expect("the updated card must remain readable") else {
        panic!("expected card detail output")
    };
    assert_eq!(after_update.title.as_deref(), Some(updated_title.as_str()));
    assert_eq!(
        after_update.parent_id.as_deref(),
        Some(parent.card_id.as_str())
    );
    assert_eq!(after_update.description.as_deref(), Some("Updated details"));
    assert_eq!(after_update.color.as_deref(), Some("#12aBcF"));
    assert_eq!(after_update.label_ids, [label_id.clone()]);
    assert_eq!(after_update.requested_by.as_deref(), Some("Requester"));
    assert_eq!(after_update.assigned_by.as_deref(), Some("Dispatcher"));
    assert_eq!(
        after_update.received_at.as_deref(),
        Some("2031-01-01T00:00:00.000Z")
    );
    assert_eq!(
        after_update.start_at.as_deref(),
        Some("2031-01-02T00:00:00.000Z")
    );
    assert_eq!(
        after_update.due_at.as_deref(),
        Some("2031-01-03T00:00:00.000Z")
    );
    assert_eq!(
        after_update.end_at.as_deref(),
        Some("2031-01-04T00:00:00.000Z")
    );
    assert_eq!(
        after_update.spent_time.and_then(|value| value.as_f64()),
        Some(2.5)
    );
    assert_eq!(
        after_update.sort.and_then(|value| value.as_f64()),
        Some(73.25)
    );
    assert_eq!(after_update.is_overtime, Some(false));
    assert_eq!(after_update.due_complete, Some(true));

    let submitted_long_title = "x".repeat(1001);
    let CommandSuccess::CardUpdated(truncated_title_update) = execute_live_card(
        &app,
        &server,
        &[
            "update",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &created.card_id,
            "--title",
            &submitted_long_title,
        ],
    )
    .await
    .expect("the CLI must submit a card title that Wekan truncates") else {
        panic!("expected card update output")
    };
    assert_eq!(
        truncated_title_update.submitted_fields,
        [wekan_cli::command_result::CardSubmittedField::Title]
    );
    let CommandSuccess::CardShown(truncated_title_card) = execute_live_card(
        &app,
        &server,
        &[
            "get",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &created.card_id,
        ],
    )
    .await
    .expect("the card with Wekan's truncated title must remain readable") else {
        panic!("expected card detail output")
    };
    assert_eq!(
        truncated_title_card.title.as_deref(),
        Some("x".repeat(1000).as_str())
    );

    let ignored_zero_spent_time = execute_live_card(
        &app,
        &server,
        &[
            "update",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &created.card_id,
            "--spent-time",
            "0",
        ],
    )
    .await
    .expect_err("Wekan v11.06 must expose its ignored spentTime zero behavior");
    assert_eq!(ignored_zero_spent_time.code(), ErrorCode::NotFound);
    assert_eq!(
        ignored_zero_spent_time.details().outcome_unknown,
        Some(true)
    );
    let CommandSuccess::CardShown(after_ignored_zero) = execute_live_card(
        &app,
        &server,
        &[
            "get",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &created.card_id,
        ],
    )
    .await
    .expect("the card must remain readable after Wekan ignores spentTime zero") else {
        panic!("expected card detail output")
    };
    assert_eq!(
        after_ignored_zero
            .spent_time
            .and_then(|value| value.as_f64()),
        Some(2.5)
    );

    let CommandSuccess::CardUpdated(cleared) = execute_live_card(
        &app,
        &server,
        &[
            "update",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &created.card_id,
            "--clear-labels",
            "--clear-members",
            "--clear-assignees",
            "--clear-received-at",
            "--clear-start-at",
            "--clear-due-at",
            "--clear-end-at",
            "--due-complete",
            "false",
        ],
    )
    .await
    .expect("the supported collection and date clears must succeed") else {
        panic!("expected card clear update output")
    };
    assert_eq!(
        cleared.submitted_fields,
        vec![
            wekan_cli::command_result::CardSubmittedField::LabelIds,
            wekan_cli::command_result::CardSubmittedField::ReceivedAt,
            wekan_cli::command_result::CardSubmittedField::StartAt,
            wekan_cli::command_result::CardSubmittedField::DueAt,
            wekan_cli::command_result::CardSubmittedField::EndAt,
            wekan_cli::command_result::CardSubmittedField::Members,
            wekan_cli::command_result::CardSubmittedField::Assignees,
            wekan_cli::command_result::CardSubmittedField::DueComplete,
        ]
    );
    let CommandSuccess::CardShown(after_clear) = execute_live_card(
        &app,
        &server,
        &[
            "get",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &created.card_id,
        ],
    )
    .await
    .expect("the cleared card must remain readable") else {
        panic!("expected card detail output")
    };
    assert!(after_clear.label_ids.is_empty());
    assert!(after_clear.members.is_empty());
    assert!(after_clear.assignees.is_empty());
    assert_eq!(after_clear.received_at, None);
    assert_eq!(after_clear.start_at, None);
    assert_eq!(after_clear.due_at, None);
    assert_eq!(after_clear.end_at, None);
    assert_eq!(after_clear.due_complete, Some(false));

    let ignored_parent = live_api_json(
        &http,
        &canonical_server,
        &token,
        reqwest::Method::POST,
        &format!("api/boards/{}/lists/{}/cards", board.board_id, list.list_id),
        Some(serde_json::json!({
            "title": "Ignored parent probe",
            "swimlaneId": board.default_swimlane_id,
            "parentId": parent.card_id
        })),
    )
    .await;
    let ignored_parent_id = required_id(&ignored_parent, "ignored-parent card");
    let ignored_parent_card = live_api_json(
        &http,
        &canonical_server,
        &token,
        reqwest::Method::GET,
        &format!(
            "api/boards/{}/lists/{}/cards/{ignored_parent_id}",
            board.board_id, list.list_id
        ),
        None,
    )
    .await;
    assert!(
        ignored_parent_card
            .get("parentId")
            .and_then(serde_json::Value::as_str)
            .is_none_or(str::is_empty),
        "single-card create unexpectedly persisted body parentId: {ignored_parent_card}"
    );

    let broken_value_url = url::Url::parse(&canonical_server)
        .unwrap()
        .join(&format!(
            "api/boards/{}/lists/{}/cards/{ignored_parent_id}",
            board.board_id, list.list_id
        ))
        .unwrap();
    let ignored_values = http
        .put(broken_value_url.clone())
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .json(&serde_json::json!({"sort": 0, "isOverTime": false}))
        .send()
        .await
        .expect("the ignored-value probe must complete");
    assert_eq!(ignored_values.status(), reqwest::StatusCode::NOT_FOUND);
    let misspelled_write = http
        .put(broken_value_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .json(&serde_json::json!({"isOverTime": true}))
        .send()
        .await
        .expect("the misspelled isOverTime probe must complete");
    assert!(misspelled_write.status().is_success());
    let misspelled_card = live_api_json(
        &http,
        &canonical_server,
        &token,
        reqwest::Method::GET,
        &format!(
            "api/boards/{}/lists/{}/cards/{ignored_parent_id}",
            board.board_id, list.list_id
        ),
        None,
    )
    .await;
    assert_eq!(misspelled_card["isOverTime"], serde_json::Value::Null);
    assert_eq!(misspelled_card["isOvertime"], false);

    let wrong_list_parent = live_api_json(
        &http,
        &canonical_server,
        &token,
        reqwest::Method::POST,
        &format!("api/boards/{}/lists/{}/cards", board.board_id, list.list_id),
        Some(serde_json::json!({
            "title": "Wrong-list parent",
            "swimlaneId": board.default_swimlane_id
        })),
    )
    .await;
    let wrong_list_parent_id = required_id(&wrong_list_parent, "wrong-list parent");
    let wrong_list_child = live_api_json(
        &http,
        &canonical_server,
        &token,
        reqwest::Method::POST,
        &format!("api/boards/{}/lists/{}/cards", board.board_id, list.list_id),
        Some(serde_json::json!({
            "title": "Wrong-list child",
            "swimlaneId": board.default_swimlane_id
        })),
    )
    .await;
    let wrong_list_child_id = required_id(&wrong_list_child, "wrong-list child");
    live_api_json(
        &http,
        &canonical_server,
        &token,
        reqwest::Method::PUT,
        &format!(
            "api/boards/{}/lists/{}/cards/{wrong_list_child_id}",
            board.board_id, list.list_id
        ),
        Some(serde_json::json!({"parentId": wrong_list_parent_id})),
    )
    .await;
    let wrong_delete = live_api_json(
        &http,
        &canonical_server,
        &token,
        reqwest::Method::DELETE,
        &format!(
            "api/boards/{}/lists/wrong-list/cards/{wrong_list_parent_id}",
            board.board_id
        ),
        None,
    )
    .await;
    assert_eq!(
        required_id(&wrong_delete, "wrong-list deletion"),
        wrong_list_parent_id
    );
    let surviving_parent = live_api_json(
        &http,
        &canonical_server,
        &token,
        reqwest::Method::GET,
        &format!(
            "api/boards/{}/lists/{}/cards/{wrong_list_parent_id}",
            board.board_id, list.list_id
        ),
        None,
    )
    .await;
    assert_eq!(surviving_parent["_id"], wrong_list_parent_id);
    wait_for_live_card_absent(
        &http,
        &canonical_server,
        &token,
        &board.board_id,
        &list.list_id,
        &wrong_list_child_id,
    )
    .await;

    let CommandSuccess::CardDeleted(deleted) = execute_live_card(
        &app,
        &server,
        &[
            "delete",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &created.card_id,
            "--yes",
        ],
    )
    .await
    .expect("hard card deletion must succeed") else {
        panic!("expected card deletion output")
    };
    assert!(deleted.deleted);
    assert_eq!(deleted.delete_mode, CardDeleteMode::Hard);
    let missing = execute_live_card(
        &app,
        &server,
        &[
            "get",
            "--board",
            &board.board_id,
            "--list",
            &list.list_id,
            &created.card_id,
        ],
    )
    .await
    .expect_err("the hard-deleted card must be reported as missing");
    assert_eq!(missing.code(), ErrorCode::NotFound);
    assert_eq!(missing.details().http_status, Some(200));
    assert_eq!(missing.details().wekan_status_code, Some(404));

    for card_id in [parent.card_id, ignored_parent_id, wrong_list_parent_id]
        .into_iter()
        .chain(permission_cleanup_ids)
    {
        live_api_json(
            &http,
            &canonical_server,
            &token,
            reqwest::Method::DELETE,
            &format!(
                "api/boards/{}/lists/{}/cards/{card_id}",
                board.board_id, list.list_id
            ),
            None,
        )
        .await;
    }
}
