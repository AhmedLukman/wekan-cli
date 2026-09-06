use super::support::*;
#[tokio::test]
#[ignore = "requires a fresh user-started Wekan v11.06 stack and WEKAN_API_E2E_URL"]
async fn raw_api_escape_hatch_matches_wekan_v11_06() {
    let server = env::var("WEKAN_API_E2E_URL")
        .expect("set WEKAN_API_E2E_URL to a fresh isolated Wekan v11.06 server URL");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock must be after the Unix epoch")
        .as_millis();
    let username = format!("wekan_cli_raw_api_admin_{nonce}");
    let email = format!("{username}@example.test");
    let password = format!("Wekan-raw-api-e2e-{nonce}!");
    let directory = TestDirectory::new("raw-api-escape-hatch");
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
        .expect("the first account on the raw API stack must register as administrator")
    else {
        panic!("expected registration output")
    };

    let CommandSuccess::BoardCreated(board) = execute_live_board(
        &app,
        &server,
        &["create", "--title", &format!("Raw API {nonce}")],
    )
    .await
    .expect("the raw API test board must be created") else {
        panic!("expected board creation output")
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
    .expect("the raw API test list must be created") else {
        panic!("expected list creation output")
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
            "Raw API card",
            "--swimlane-id",
            &board.default_swimlane_id,
        ],
    )
    .await
    .expect("the raw API test card must be created") else {
        panic!("expected card creation output")
    };

    let canonical_server = ServerUrl::parse(&server)
        .expect("the raw API E2E server URL must be canonicalizable")
        .as_str()
        .to_owned();
    let admin_token = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .map(|(_, _, _, token, _)| token.clone())
        .expect("the raw API administrator credential must be stored");
    let (_member_directory, member_app, _member_id) = register_live_board_role(
        &server,
        &canonical_server,
        &admin_token,
        &board.board_id,
        nonce,
        "normal",
    )
    .await;

    let checklist_path = format!(
        "/api/boards/{}/cards/{}/checklists",
        board.board_id, card.card_id
    );
    let created = execute_live_api_output(
        &app,
        &server,
        &[
            "request",
            "POST",
            &checklist_path,
            "--json",
            r#"{"title":"Release","items":["Verify"]}"#,
            "--yes",
        ],
        OutputFormat::Json,
    )
    .await;
    let created_json: serde_json::Value = serde_json::from_slice(&created).unwrap();
    let checklist_id = created_json["data"]["body"]["value"]["_id"]
        .as_str()
        .expect("checklist creation must return its ID")
        .to_owned();

    wait_for_live_raw_checklist(&app, &server, &checklist_path, &checklist_id, "Release").await;
    wait_for_live_raw_checklist(
        &member_app,
        &server,
        &checklist_path,
        &checklist_id,
        "Release",
    )
    .await;

    let checklist_delete_path = format!("{checklist_path}/{checklist_id}");
    execute_live_api_output(
        &app,
        &server,
        &["request", "DELETE", &checklist_delete_path, "--yes"],
        OutputFormat::Json,
    )
    .await;
    wait_for_live_raw_checklist_absent(&app, &server, &checklist_path, &checklist_id).await;

    let no_auth = execute_live_api_output(
        &app,
        &server,
        &["request", "GET", "/api/boards", "--no-auth"],
        OutputFormat::Json,
    )
    .await;
    let no_auth: serde_json::Value = serde_json::from_slice(&no_auth).unwrap();
    assert_eq!(no_auth["data"]["http_status"], 200);

    let export_path = format!(
        "/api/boards/{}/lists/{}/cards/{}/exportPDF",
        board.board_id, list.list_id, card.card_id
    );
    let exported = execute_live_api_output(
        &app,
        &server,
        &["request", "GET", &export_path, "--auth-token-query"],
        OutputFormat::Raw,
    )
    .await;
    assert!(exported.starts_with(b"%PDF"), "card export must be a PDF");

    let (exit, stdout, stderr) = execute_live_api_result(
        &app,
        &server,
        &["request", "GET", &export_path, "--no-auth"],
        OutputFormat::Json,
    )
    .await;
    assert_ne!(exit, std::process::ExitCode::SUCCESS);
    assert!(stdout.is_empty());
    let denied: serde_json::Value = serde_json::from_slice(&stderr)
        .expect("unauthenticated private export must produce structured error output");
    assert_eq!(denied["error"]["code"], "authentication_rejected");
    assert_eq!(denied["error"]["details"]["http_status"], 401);

    execute_live_board(&app, &server, &["delete", &board.board_id, "--yes"])
        .await
        .expect("the raw API test board must be removed");
}
