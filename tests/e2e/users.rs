use super::support::*;
async fn verify_repeated_ownership(
    server: &str,
    first: &live::LiveApp,
    first_id: &str,
    second: &live::LiveApp,
    second_id: &str,
) {
    let CommandSuccess::BoardCreated(board) =
        execute_live_board(first, server, &["create", "--title", "Repeated ownership"])
            .await
            .unwrap()
    else {
        panic!("expected board creation")
    };
    let board_id = board.board_id.as_str();
    execute_live_board(
        first,
        server,
        &[
            "member", "add", second_id, "--board", board_id, "--role", "admin",
        ],
    )
    .await
    .unwrap();

    // The first transfer leaves the original owner's inactive entry ahead of the
    // active administrator entry they acquired. The return transfer changes only the first.
    for (caller, target, expected_first) in
        [(first, second_id, first_id), (second, first_id, second_id)]
    {
        let CommandSuccess::UserOwnershipTaken(result) =
            execute_live_user(caller, server, &["take-ownership", target, "--yes"])
                .await
                .unwrap()
        else {
            panic!("expected ownership result")
        };
        assert!(result.boards.iter().any(|b| b.board_id == board_id));
        let CommandSuccess::BoardMembers(result) =
            execute_live_board(first, server, &["member", "list", "--board", board_id])
                .await
                .unwrap()
        else {
            panic!("expected board members")
        };
        let entries: Vec<_> = result
            .members
            .iter()
            .map(|m| (m.user_id.as_str(), m.is_active, m.is_admin))
            .collect();
        assert_eq!(
            entries,
            [(expected_first, false, false), (first_id, true, true)]
        );
    }
    execute_live_board(first, server, &["delete", board_id, "--yes"])
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "destructive: requires a fresh isolated Wekan v13.28 stack and WEKAN_USER_E2E_URL"]
async fn complete_user_flow_matches_wekan_v13_28() {
    let server = env::var("WEKAN_USER_E2E_URL")
        .expect("set WEKAN_USER_E2E_URL to a fresh isolated Wekan v13.28 server URL");
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

    let http = reqwest::Client::new();
    let missing_user = format!("wekan_cli_missing_user_{nonce}");
    for args in [
        vec!["get", &missing_user],
        vec!["update", &missing_user],
        vec!["enable-login", &missing_user],
        vec!["disable-login", &missing_user, "--yes"],
    ] {
        let error = execute_live_user(&admin_app, &server, &args)
            .await
            .expect_err("an unknown user must return HTTP 404");
        assert_eq!(error.code(), ErrorCode::NotFound);
        assert_eq!(error.details().http_status, Some(404));
        assert_eq!(
            error.details().server_error.as_deref(),
            Some("User not found")
        );
    }

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

        assert!(!created.user_id.is_empty());
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
    let owner_token = owner_app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .3
        .clone();
    let owner_board = live_api_json(
        &http,
        &canonical_server,
        &owner_token,
        reqwest::Method::POST,
        "api/boards",
        Some(serde_json::json!({
            "title": format!("Owner board {nonce}"),
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
    .expect("the verified card filters must work against v13.28") else {
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
    let rejected = member_app
        .execute(disabled_member_login.command)
        .await
        .expect_err("a login-disabled account must not start a REST session");
    assert_eq!(rejected.code(), ErrorCode::LoginRejected);
    assert_eq!(rejected.details().http_status, Some(401));
    assert!(
        member_app
            .credential_store()
            .credential
            .lock()
            .unwrap()
            .is_none()
    );

    let CommandSuccess::UserLoginChanged(enabled) =
        execute_live_user(&admin_app, &server, &["enable-login", &member_id])
            .await
            .expect("site admin must be able to restore login")
    else {
        panic!("expected enable-login output")
    };
    assert_eq!(enabled.user.login_disabled, Some(false));
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
    assert_eq!(member_after.login_disabled, Some(false));

    let CommandSuccess::BoardCreated(inactive_membership_board) = execute_live_board(
        &member_app,
        &server,
        &["create", "--title", "Former administrator"],
    )
    .await
    .unwrap() else {
        panic!("expected board creation")
    };
    let inactive_board_id = inactive_membership_board.board_id;
    // Wekan's permission setter can update the role of an inactive membership.
    for (action, role) in [
        ("add", Some("normal")),
        ("remove", None),
        ("update", Some("admin")),
    ] {
        let mut args = vec!["member", action, &owner_id, "--board", &inactive_board_id];
        if let Some(role) = role {
            args.extend(["--role", role]);
        }
        execute_live_board(&member_app, &server, &args)
            .await
            .unwrap();
    }
    execute_live_board(
        &member_app,
        &server,
        &[
            "member",
            "add",
            &admin_id,
            "--board",
            &inactive_board_id,
            "--role",
            "normal",
        ],
    )
    .await
    .unwrap();
    let CommandSuccess::BoardMembers(before) = execute_live_board(
        &member_app,
        &server,
        &["member", "list", "--board", &inactive_board_id],
    )
    .await
    .unwrap() else {
        panic!("expected board memberships before reassignment")
    };
    assert!(
        before
            .members
            .iter()
            .any(|m| m.user_id == owner_id && !m.is_active && m.is_admin)
    );
    assert!(
        before
            .members
            .iter()
            .any(|m| m.user_id == admin_id && m.is_active && !m.is_admin)
    );
    let CommandSuccess::UserOwnershipTaken(transfer) =
        execute_live_user(&admin_app, &server, &["take-ownership", &owner_id, "--yes"])
            .await
            .expect("site admin must be able to take board ownership")
    else {
        panic!("expected ownership-transfer output")
    };
    assert!(
        !transfer
            .boards
            .iter()
            .any(|board| board.board_id == admin_board_id)
    );
    for (board_id, expected_active, board_admin) in [
        (&owner_board_id, true, &admin_app),
        (&inactive_board_id, false, &member_app),
    ] {
        assert!(
            transfer
                .boards
                .iter()
                .any(|board| board.board_id == *board_id)
        );
        let CommandSuccess::BoardMembers(members) = execute_live_board(
            board_admin,
            &server,
            &["member", "list", "--board", board_id],
        )
        .await
        .unwrap() else {
            panic!("expected board memberships")
        };
        assert!(members.members.iter().any(|member| {
            member.user_id == admin_id && member.is_admin && member.is_active == expected_active
        }));
        if !expected_active {
            assert!(
                !members
                    .members
                    .iter()
                    .any(|m| m.user_id == admin_id && m.is_active)
            );
        }
        assert!(
            !members
                .members
                .iter()
                .any(|member| member.user_id == owner_id && member.is_admin)
        );
    }

    execute_live_board(
        &member_app,
        &server,
        &["delete", &inactive_board_id, "--yes"],
    )
    .await
    .unwrap();

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
        let repeat = execute_live_user(&admin_app, &server, &["delete", user_id, "--yes"])
            .await
            .expect_err("a deleted user no longer matches the native selector");
        assert_eq!(repeat.details().http_status, Some(404));
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

    let denied = execute_live_user(&admin_app, &server, &["delete", &admin_id, "--yes"])
        .await
        .expect_err("default account settings prohibit self-deletion");
    assert_eq!(denied.code(), ErrorCode::PermissionDenied);
    assert_eq!(denied.details().http_status, Some(403));
    assert_eq!(
        denied.details().server_reason.as_deref(),
        Some("Deleting your own account is not allowed")
    );
    let current = execute_live_user(&admin_app, &server, &["current"])
        .await
        .unwrap();
    assert!(matches!(current, CommandSuccess::UserCurrent(user) if user.user_id == admin_id));

    // Enable the site's self-deletion preference and provision another site administrator.
    let backup_username = format!("backup_{nonce}");
    let CommandSuccess::UserCreated(backup) = execute_live_user(
        &admin_app,
        &server,
        &[
            "create",
            "--username",
            &backup_username,
            "--email",
            &format!("backup_{nonce}@example.test"),
            "--password-stdin",
        ],
    )
    .await
    .unwrap() else {
        panic!("expected backup administrator fixture")
    };
    super::support::attachment_fixtures::fixture_command(
        "mongodb",
        "mongosh",
        r#"
        const w = db.getSiblingDB('wekan');
        w.accountSettings.updateOne({_id:'accounts-allowUserDelete'}, {$set:{booleanValue:true}}, {upsert:true});
        w.users.updateOne({_id:p.backup}, {$set:{isAdmin:true}});
        print('{}');
    "#,
        &serde_json::json!({"backup":backup.user_id}),
    );
    let backup_directory = TestDirectory::new("complete-user-backup");
    let backup_app = App::with_profile_store(
        ServerSelection::new(Some(server.clone()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password)),
        FileProfileStore::at(backup_directory.path().to_owned()),
    );
    live::run(
        &backup_app,
        &server,
        &[
            "auth",
            "login",
            "--username",
            &backup_username,
            "--password-stdin",
        ],
    )
    .await
    .unwrap();
    verify_repeated_ownership(&server, &admin_app, &admin_id, &backup_app, &backup.user_id).await;

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
