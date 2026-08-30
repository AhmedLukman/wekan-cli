use super::support::*;
#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v11.06 stack and WEKAN_USER_E2E_URL"]
async fn complete_user_flow_matches_wekan_v11_06() {
    let server = env::var("WEKAN_USER_E2E_URL")
        .expect("set WEKAN_USER_E2E_URL to a fresh isolated Wekan v11.06 server URL");
    let canonical_server = ServerUrl::parse(&server)
        .expect("the live-test Wekan URL must be valid")
        .as_str()
        .to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock must be after the Unix epoch")
        .as_millis();
    let admin_username = format!("wekan_cli_user_admin_{nonce}");
    let admin_email = format!("{admin_username}@example.test");
    let owner_username = format!("wekan_cli_user_owner_{nonce}");
    let owner_email = format!("{owner_username}@example.test");
    let member_username = format!("wekan_cli_user_member_{nonce}");
    let member_email = format!("{member_username}@example.test");
    let password = format!("Wekan-user-e2e-{nonce}!");

    let admin_directory = TestDirectory::new("complete-user-admin");
    let admin_app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password.clone())),
        FileProfileStore::at(admin_directory.path().to_owned()),
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
    let CommandSuccess::Registration(admin_registration) = admin_app
        .execute(registration.command)
        .await
        .expect("the first account on a fresh stack must register as administrator")
    else {
        panic!("expected registration output")
    };
    let admin_id = admin_registration.user_id;
    let admin_token = admin_app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .map(|(_, _, _, token, _)| token.clone())
        .expect("admin registration must store a token");

    let CommandSuccess::UserCurrent(current) = execute_live_user(&admin_app, &server, &["current"])
        .await
        .expect("user current must succeed")
    else {
        panic!("expected current-user output")
    };
    assert_eq!(current.user_id, admin_id);
    assert_eq!(current.username.as_deref(), Some(admin_username.as_str()));
    assert_eq!(current.is_admin, Some(true));

    for (username, email) in [
        (&owner_username, &owner_email),
        (&member_username, &member_email),
    ] {
        let CommandSuccess::UserCreated(created) = execute_live_user(
            &admin_app,
            &server,
            &[
                "create",
                "--username",
                username,
                "--email",
                email,
                "--password-stdin",
            ],
        )
        .await
        .expect("site admin must be able to create temporary users") else {
            panic!("expected create-user output")
        };
        assert!(created.created);
        assert_eq!(created.user_id, None);
        assert_eq!(
            created.warning,
            Some(wekan_cli::command_result::UserCreateWarning::UserIdUnavailableInWekanV1106)
        );
    }

    let owner_id = wait_for_live_user(&admin_app, &server, &owner_username).await;
    let member_id = wait_for_live_user(&admin_app, &server, &member_username).await;

    let CommandSuccess::UserShown(owner) =
        execute_live_user(&admin_app, &server, &["get", &owner_username])
            .await
            .expect("site admin must resolve a user by username")
    else {
        panic!("expected user-detail output")
    };
    assert_eq!(owner.user_id, owner_id);

    let http = reqwest::Client::new();
    let owner_board = live_api_json(
        &http,
        &canonical_server,
        &admin_token,
        reqwest::Method::POST,
        "api/boards",
        Some(serde_json::json!({
            "title": format!("Owner board {nonce}"),
            "owner": owner_id,
            "permission": "private"
        })),
    )
    .await;
    let owner_board_id = required_id(&owner_board, "owner board");

    let CommandSuccess::UserBoards(admin_view) =
        execute_live_user(&admin_app, &server, &["boards", &owner_id])
            .await
            .expect("site admin must be able to list another user's boards")
    else {
        panic!("expected user-board output")
    };
    assert!(
        admin_view
            .boards
            .iter()
            .any(|board| board.board_id == owner_board_id)
    );

    let owner_directory = TestDirectory::new("complete-user-owner");
    let owner_app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password.clone())),
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
    owner_app
        .execute(owner_login.command)
        .await
        .expect("the temporary owner must be able to log in");
    let CommandSuccess::UserBoards(self_view) =
        execute_live_user(&owner_app, &server, &["boards", &owner_id])
            .await
            .expect("an ordinary user must be able to list their own boards")
    else {
        panic!("expected self-board output")
    };
    assert!(
        self_view
            .boards
            .iter()
            .any(|board| board.board_id == owner_board_id)
    );
    let denied = execute_live_user(&owner_app, &server, &["list"])
        .await
        .expect_err("an ordinary user must not list all users");
    assert_eq!(denied.code(), ErrorCode::PermissionDenied);

    let member_directory = TestDirectory::new("complete-user-member");
    let member_app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password.clone())),
        FileProfileStore::at(member_directory.path().to_owned()),
    );
    let member_login = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "login",
        "--username",
        &member_username,
        "--password-stdin",
    ])
    .unwrap();
    member_app
        .execute(member_login.command)
        .await
        .expect("the temporary member must be able to log in before disabling");
    let CommandSuccess::UserCurrent(member_before) =
        execute_live_user(&member_app, &server, &["current"])
            .await
            .expect("the member session must be readable before disabling login")
    else {
        panic!("expected current-user output")
    };
    assert_eq!(member_before.user_id, member_id);

    let admin_board = live_api_json(
        &http,
        &canonical_server,
        &admin_token,
        reqwest::Method::POST,
        "api/boards",
        Some(serde_json::json!({
            "title": format!("Admin card board {nonce}"),
            "owner": admin_id,
            "permission": "private"
        })),
    )
    .await;
    let admin_board_id = required_id(&admin_board, "admin board");
    let swimlane_id = admin_board["defaultSwimlaneId"]
        .as_str()
        .expect("board creation must return the default swimlane ID");
    let list = live_api_json(
        &http,
        &canonical_server,
        &admin_token,
        reqwest::Method::POST,
        &format!("api/boards/{admin_board_id}/lists"),
        Some(serde_json::json!({"title": "E2E list", "swimlaneId": swimlane_id})),
    )
    .await;
    let list_id = required_id(&list, "list");
    let card = live_api_json(
        &http,
        &canonical_server,
        &admin_token,
        reqwest::Method::POST,
        &format!("api/boards/{admin_board_id}/lists/{list_id}/cards"),
        Some(serde_json::json!({
            "title": format!("Due card {nonce}"),
            "swimlaneId": swimlane_id,
            "members": [admin_id],
            "dueAt": "2030-06-15T12:00:00Z"
        })),
    )
    .await;
    let card_id = required_id(&card, "card");

    let CommandSuccess::UserCards(cards) = execute_live_user(
        &admin_app,
        &server,
        &[
            "cards",
            "--due",
            "--from",
            "2030-01-01T00:00:00Z",
            "--to",
            "2030-12-31T23:59:59Z",
        ],
    )
    .await
    .expect("the verified card filters must work against v11.06") else {
        panic!("expected user-card output")
    };
    assert!(cards.cards.iter().any(|card| card.card_id == card_id));
    let CommandSuccess::UserCards(out_of_range) = execute_live_user(
        &admin_app,
        &server,
        &[
            "cards",
            "--from",
            "2031-01-01T00:00:00Z",
            "--to",
            "2031-12-31T23:59:59Z",
        ],
    )
    .await
    .expect("date-only filters must imply due-date filtering") else {
        panic!("expected filtered user-card output")
    };
    assert!(
        !out_of_range
            .cards
            .iter()
            .any(|card| card.card_id == card_id)
    );

    let CommandSuccess::UserLoginChanged(disabled) =
        execute_live_user(&admin_app, &server, &["disable-login", &member_id, "--yes"])
            .await
            .expect("site admin must be able to disable login")
    else {
        panic!("expected disable-login output")
    };
    assert_eq!(disabled.user.login_disabled, Some(true));

    let disabled_session = execute_live_user(&member_app, &server, &["current"])
        .await
        .expect_err("disabling login must invalidate the member's existing session");
    assert_eq!(disabled_session.code(), ErrorCode::AuthenticationRejected);

    let member_local_logout = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "logout",
        "--local-only",
        "--yes",
    ])
    .unwrap();
    member_app
        .execute(member_local_logout.command)
        .await
        .expect("the disabled member's invalidated local credential must be removable");
    let disabled_member_login = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "login",
        "--username",
        &member_username,
        "--password-stdin",
    ])
    .unwrap();
    member_app
        .execute(disabled_member_login.command)
        .await
        .expect("the pinned v11.06 REST-login defect must remain visible");
    let CommandSuccess::UserCurrent(disabled_after_relogin) =
        execute_live_user(&member_app, &server, &["current"])
            .await
            .expect("the REST login token minted for a login-disabled user must authenticate")
    else {
        panic!("expected current-user output")
    };
    assert_eq!(disabled_after_relogin.user_id, member_id);
    assert_eq!(disabled_after_relogin.login_disabled, Some(true));

    let disabled_login_cleanup = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "logout",
        "--local-only",
        "--yes",
    ])
    .unwrap();
    member_app
        .execute(disabled_login_cleanup.command)
        .await
        .expect("the defect-demonstration credential must be removable");

    let CommandSuccess::UserLoginChanged(enabled) =
        execute_live_user(&admin_app, &server, &["enable-login", &member_id])
            .await
            .expect("site admin must be able to restore login")
    else {
        panic!("expected enable-login output")
    };
    assert_eq!(enabled.user.login_disabled, None);
    let member_relogin = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "login",
        "--username",
        &member_username,
        "--password-stdin",
    ])
    .unwrap();
    member_app
        .execute(member_relogin.command)
        .await
        .expect("an enabled member must be able to start a new session");
    let CommandSuccess::UserCurrent(member_after) =
        execute_live_user(&member_app, &server, &["current"])
            .await
            .expect("the re-enabled member session must be readable")
    else {
        panic!("expected current-user output")
    };
    assert_eq!(member_after.user_id, member_id);
    assert_eq!(member_after.login_disabled, None);

    let CommandSuccess::UserOwnershipTaken(transfer) =
        execute_live_user(&admin_app, &server, &["take-ownership", &owner_id, "--yes"])
            .await
            .expect("site admin must be able to take board ownership")
    else {
        panic!("expected ownership-transfer output")
    };
    assert!(
        transfer
            .boards
            .iter()
            .any(|board| board.board_id == owner_board_id)
    );
    wait_for_live_board_admin_membership(&admin_app, &server, &owner_board_id).await;

    for user_id in [&member_id, &owner_id] {
        let CommandSuccess::UserDeleted(deleted) =
            execute_live_user(&admin_app, &server, &["delete", user_id, "--yes"])
                .await
                .expect("site admin must be able to delete temporary users")
        else {
            panic!("expected user-delete output")
        };
        assert!(deleted.deleted);
        assert!(!deleted.deleted_current_user);
        wait_for_live_user_absent(&admin_app, &server, user_id).await;
    }
    let member_local_cleanup = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "logout",
        "--local-only",
        "--yes",
    ])
    .unwrap();
    member_app
        .execute(member_local_cleanup.command)
        .await
        .expect("the deleted temporary member's local credential must be cleaned up");
    let owner_local_cleanup = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "auth",
        "logout",
        "--local-only",
        "--yes",
    ])
    .unwrap();
    owner_app
        .execute(owner_local_cleanup.command)
        .await
        .expect("the deleted temporary owner's local credential must be cleaned up");

    for board_id in [&owner_board_id, &admin_board_id] {
        live_api_json(
            &http,
            &canonical_server,
            &admin_token,
            reqwest::Method::DELETE,
            &format!("api/boards/{board_id}"),
            None,
        )
        .await;
    }

    let CommandSuccess::UserDeleted(self_deleted) =
        execute_live_user(&admin_app, &server, &["delete", &admin_id, "--yes"])
            .await
            .expect("the administrator must be able to delete their own test account")
    else {
        panic!("expected self-delete output")
    };
    assert!(self_deleted.deleted);
    assert!(self_deleted.deleted_current_user);
    assert!(self_deleted.local_credential_removed);
    assert!(!self_deleted.credential_stored);
    assert!(
        admin_app
            .credential_store()
            .credential
            .lock()
            .unwrap()
            .is_none()
    );
    assert_token_is_rejected(&canonical_server, &admin_token).await;
}
