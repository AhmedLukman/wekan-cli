use std::{
    env, fs,
    io::Write,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use reqwest::header::AUTHORIZATION;
use secrecy::{ExposeSecret, SecretString};
use time::{Duration, OffsetDateTime};
use wekan_cli::{
    app::App,
    cli::Cli,
    client::ServerUrl,
    command_result::{
        CardDeleteMode, CommandSuccess, ListDeleteMode, LogoutScope, SwimlaneDeleteMode,
    },
    config::{
        ServerSelection,
        profiles::{FileProfileStore, Profile, ProfileDocument, ProfileStore},
    },
    credentials::{
        CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialRecord,
        CredentialStore, CredentialTarget, KeyringCredentialStore, LoginSecretMode, LoginSecrets,
        SecretInputProvider,
    },
    error::{AppError, ErrorCode},
    output::{OutputFormat, write_success},
};

type CapturedCredential = (String, String, String, String, OffsetDateTime);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = env::temp_dir().join(format!(
            "wekan-cli-e2e-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("the E2E profile directory must be created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Default)]
struct CapturingStore {
    credential: Mutex<Option<CapturedCredential>>,
}

impl CredentialStore for CapturingStore {
    fn check_available(&self, _target: &CredentialTarget) -> Result<(), CredentialError> {
        Ok(())
    }

    fn exists(&self, target: &CredentialTarget) -> Result<bool, CredentialError> {
        Ok(self
            .credential
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|(stored_account, ..)| stored_account == target.account()))
    }

    fn load(&self, target: &CredentialTarget) -> Result<Option<CredentialRecord>, CredentialError> {
        Ok(self
            .credential
            .lock()
            .unwrap()
            .as_ref()
            .filter(|(stored_account, ..)| stored_account == target.account())
            .map(|(_, server, user_id, token, token_expires)| {
                CredentialRecord::new(
                    server.clone(),
                    user_id.clone(),
                    SecretString::from(token.clone()),
                    *token_expires,
                )
            }))
    }

    fn create(
        &self,
        target: &CredentialTarget,
        record: &CredentialRecord,
    ) -> Result<CredentialCreateOutcome, CredentialError> {
        let mut credential = self.credential.lock().unwrap();
        if credential
            .as_ref()
            .is_some_and(|(stored_account, ..)| stored_account == target.account())
        {
            return Ok(CredentialCreateOutcome::AlreadyExists);
        }
        *credential = Some((
            target.account().to_owned(),
            record.server_url().to_owned(),
            record.user_id().to_owned(),
            record.token().expose_secret().to_owned(),
            record.token_expires(),
        ));
        Ok(CredentialCreateOutcome::Created)
    }

    fn delete(&self, target: &CredentialTarget) -> Result<bool, CredentialError> {
        let mut credential = self.credential.lock().unwrap();
        if credential
            .as_ref()
            .is_some_and(|(stored_account, ..)| stored_account == target.account())
        {
            credential.take();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn delete_if_matches(
        &self,
        target: &CredentialTarget,
        expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        let mut credential = self.credential.lock().unwrap();
        let Some((account, server, user_id, token, token_expires)) = credential.as_ref() else {
            return Ok(CredentialDeleteOutcome::Absent);
        };
        if account != target.account() {
            return Ok(CredentialDeleteOutcome::Absent);
        }
        let current = CredentialRecord::new(
            server.clone(),
            user_id.clone(),
            SecretString::from(token.clone()),
            *token_expires,
        );
        if !current.matches(expected) {
            return Ok(CredentialDeleteOutcome::Mismatch);
        }

        credential.take();
        Ok(CredentialDeleteOutcome::Removed)
    }
}

struct FixedPassword(SecretString);

impl SecretInputProvider for FixedPassword {
    fn read_new_account_password(&self, _from_stdin: bool) -> Result<SecretString, AppError> {
        Ok(self.0.clone())
    }

    fn read_login_secrets(&self, _mode: LoginSecretMode) -> Result<LoginSecrets, AppError> {
        Ok(LoginSecrets::new(self.0.clone(), None))
    }
}

#[tokio::test]
#[ignore = "requires a user-started Wekan v11.06 stack and WEKAN_E2E_URL"]
async fn authentication_flow_matches_wekan_v11_06() {
    let server = env::var("WEKAN_E2E_URL")
        .expect("set WEKAN_E2E_URL to the user-started Wekan v11.06 server URL");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock must be after the Unix epoch")
        .as_millis();
    let username = format!("wekan_cli_e2e_{}_{}", std::process::id(), nonce);
    let email = format!("{username}@example.test");
    let password = format!("Wekan-e2e-{nonce}!");
    let cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "register",
        "--username",
        &username,
        "--email",
        &email,
        "--password-stdin",
    ])
    .expect("the live-test command must parse");
    let store = CapturingStore::default();
    let passwords = FixedPassword(SecretString::from(password));
    let directory = TestDirectory::new("default-profile-auth");
    let app = App::with_profile_store(
        ServerSelection::new(
            cli.server,
            Some("default".to_owned()),
            cli.allow_insecure_http,
        ),
        store,
        passwords,
        FileProfileStore::at(directory.path().to_owned()),
    );

    let registration = app
        .execute(cli.command)
        .await
        .expect("registration must succeed against Wekan v11.06");
    let CommandSuccess::Registration(registration) = registration else {
        panic!("expected registration output")
    };
    assert_eq!(registration.profile, "default");
    assert!(registration.profile_created);
    assert!(registration.profile_active);

    let (credential_account, canonical_server, user_id, token, _) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .expect("registration must save the credential");
    assert_token_authenticates(&canonical_server, &user_id, &token).await;

    let clear_registration_cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "logout",
        "--local-only",
        "--yes",
    ])
    .expect("the post-registration cleanup command must parse");
    app.execute(clear_registration_cli.command)
        .await
        .expect("local-only logout must permit a subsequent authentication attempt");

    let duplicate_cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "register",
        "--username",
        &username,
        "--email",
        &email,
        "--password-stdin",
    ])
    .expect("the duplicate registration command must parse");
    let duplicate_error = app
        .execute(duplicate_cli.command)
        .await
        .expect_err("duplicate registration must be rejected");
    assert_eq!(duplicate_error.code(), ErrorCode::RegistrationRejected);
    assert_eq!(
        duplicate_error.details().server_error.as_deref(),
        Some("403")
    );
    assert!(
        duplicate_error
            .details()
            .server_reason
            .as_deref()
            .is_some_and(|reason| !reason.is_empty())
    );
    assert_eq!(duplicate_error.details().outcome_unknown, Some(true));

    let username_login = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "login",
        "--username",
        &username,
        "--password-stdin",
    ])
    .expect("the username login command must parse");
    app.execute(username_login.command)
        .await
        .expect("username login must succeed against Wekan v11.06");
    let (_, username_server, username_user_id, username_token, _) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .expect("username login must save the stored credential");
    assert_eq!(username_user_id, user_id);
    assert_token_authenticates(&username_server, &username_user_id, &username_token).await;

    let clear_username_login_cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "logout",
        "--local-only",
        "--yes",
    ])
    .expect("the post-username-login cleanup command must parse");
    app.execute(clear_username_login_cli.command)
        .await
        .expect("local-only logout must permit the email login");

    let email_login = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "login",
        "--email",
        &email,
        "--password-stdin",
    ])
    .expect("the email login command must parse");
    app.execute(email_login.command)
        .await
        .expect("email login must succeed against Wekan v11.06");
    let status_cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "status",
    ])
    .expect("the status command must parse");
    let status = app
        .execute(status_cli.command)
        .await
        .expect("the stored login token must pass live status validation");
    let CommandSuccess::AuthStatus(status) = status else {
        panic!("expected authentication status output")
    };
    assert_eq!(status.user.user_id, user_id);
    assert_eq!(status.user.username.as_deref(), Some(username.as_str()));
    assert!(status.user.emails.iter().any(|entry| {
        entry.address.as_deref() == Some(email.as_str()) && entry.verified == Some(false)
    }));

    let (_, email_server, email_user_id, email_token, _) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .expect("email login must save the stored credential");
    assert_eq!(email_user_id, user_id);
    assert_token_authenticates(&email_server, &email_user_id, &email_token).await;

    let local_only_cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "logout",
        "--local-only",
        "--yes",
    ])
    .expect("the local-only logout command must parse");
    let local_only = app
        .execute(local_only_cli.command)
        .await
        .expect("local-only logout must remove the stored credential");
    let CommandSuccess::Logout(local_only) = local_only else {
        panic!("expected local-only logout output")
    };
    assert_eq!(local_only.logout_scope, LogoutScope::LocalOnly);
    assert!(!local_only.remote_logout_completed);
    assert!(!local_only.credential_stored);
    assert!(local_only.local_credential_removed);
    assert!(app.credential_store().credential.lock().unwrap().is_none());
    assert_token_authenticates(&email_server, &email_user_id, &email_token).await;

    let missing_after_local_only = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "status",
    ])
    .expect("the status command after local-only logout must parse");
    let missing_error = app
        .execute(missing_after_local_only.command)
        .await
        .expect_err("local-only logout must remove the stored credential");
    assert_eq!(missing_error.code(), ErrorCode::CredentialNotFound);

    let current_login = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "login",
        "--username",
        &username,
        "--password-stdin",
    ])
    .expect("the current-token test login must parse");
    app.execute(current_login.command)
        .await
        .expect("the current-token test login must succeed");
    let (_, _, _, current_token, _) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .expect("current-token login must save its credential");

    let current_logout_cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "logout",
        "--yes",
    ])
    .expect("the current-token logout command must parse");
    let current_logout = app
        .execute(current_logout_cli.command)
        .await
        .expect("current-token logout must succeed");
    let CommandSuccess::Logout(current_logout) = current_logout else {
        panic!("expected current-token logout output")
    };
    assert_eq!(current_logout.logout_scope, LogoutScope::CurrentToken);
    assert!(current_logout.remote_logout_completed);
    assert!(!current_logout.credential_stored);
    assert!(current_logout.local_credential_removed);
    assert!(app.credential_store().credential.lock().unwrap().is_none());
    assert_token_is_rejected(&email_server, &current_token).await;
    assert_token_authenticates(&email_server, &email_user_id, &email_token).await;

    let all_login = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "login",
        "--email",
        &email,
        "--password-stdin",
    ])
    .expect("the all-token test login must parse");
    app.execute(all_login.command)
        .await
        .expect("the all-token test login must succeed");
    let (_, _, _, all_token, _) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .expect("all-token login must save its credential");

    let all_logout_cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "logout",
        "--all",
        "--yes",
    ])
    .expect("the all-token logout command must parse");
    let all_logout = app
        .execute(all_logout_cli.command)
        .await
        .expect("all-token logout must succeed");
    let CommandSuccess::Logout(all_logout) = all_logout else {
        panic!("expected all-token logout output")
    };
    assert_eq!(all_logout.logout_scope, LogoutScope::AllTokens);
    assert!(all_logout.remote_logout_completed);
    assert!(!all_logout.credential_stored);
    assert!(all_logout.local_credential_removed);
    assert!(app.credential_store().credential.lock().unwrap().is_none());
    for issued_token in [&token, &username_token, &email_token, &all_token] {
        assert_token_is_rejected(&canonical_server, issued_token).await;
    }

    *app.credential_store().credential.lock().unwrap() = Some((
        credential_account,
        canonical_server.clone(),
        user_id.clone(),
        "definitely-invalid-token".to_owned(),
        OffsetDateTime::now_utc() + Duration::days(1),
    ));
    let invalid_status_cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "status",
    ])
    .expect("the invalid-token status command must parse");
    let invalid_status = app
        .execute(invalid_status_cli.command)
        .await
        .expect_err("an invalid stored token must be rejected");
    assert_eq!(invalid_status.code(), ErrorCode::AuthenticationRejected);
    assert_eq!(invalid_status.details().http_status, Some(200));
    assert_eq!(invalid_status.details().wekan_status_code, Some(401));

    let invalid_logout_cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "logout",
        "--yes",
    ])
    .expect("the invalid-token logout command must parse");
    let invalid_logout = app
        .execute(invalid_logout_cli.command)
        .await
        .expect_err("Wekan must reject an invalid logout token");
    assert_eq!(invalid_logout.code(), ErrorCode::AuthenticationRejected);
    assert_eq!(invalid_logout.details().http_status, Some(401));
    assert_eq!(
        invalid_logout.details().remote_logout_completed,
        Some(Some(false))
    );
    assert_eq!(invalid_logout.details().credential_stored, Some(true));
    assert_eq!(
        invalid_logout.details().local_credential_removed,
        Some(false)
    );
    assert!(app.credential_store().credential.lock().unwrap().is_some());

    let cleanup_cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "logout",
        "--local-only",
        "--yes",
    ])
    .expect("the rejected-credential cleanup command must parse");
    app.execute(cleanup_cli.command)
        .await
        .expect("local-only logout must clear a rejected credential");
    assert!(app.credential_store().credential.lock().unwrap().is_none());

    let rejected_cli = Cli::try_parse_from([
        "wekan",
        "--server",
        &server,
        "--output=json",
        "auth",
        "login",
        "--username",
        &username,
        "--password-stdin",
    ])
    .expect("the rejected login command must parse");
    let rejected_directory = TestDirectory::new("rejected-default-profile-auth");
    let rejected_app = App::with_profile_store(
        ServerSelection::new(
            rejected_cli.server,
            Some("default".to_owned()),
            rejected_cli.allow_insecure_http,
        ),
        CapturingStore::default(),
        FixedPassword(SecretString::from("definitely-wrong-password".to_owned())),
        FileProfileStore::at(rejected_directory.path().to_owned()),
    );
    let rejected_error = rejected_app
        .execute(rejected_cli.command)
        .await
        .expect_err("a wrong password must be rejected");
    assert_eq!(rejected_error.code(), ErrorCode::LoginRejected);
    assert_eq!(rejected_error.details().http_status, Some(401));
    assert_eq!(
        rejected_error.details().server_error.as_deref(),
        Some("login-failed")
    );
}

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
        execute_live_list(&app, &server, &["list", &board.board_id])
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

    let CommandSuccess::ListShown(initial) =
        execute_live_list(&app, &server, &["get", &board.board_id, &created.list_id])
            .await
            .expect("the created list must be readable")
    else {
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

    let CommandSuccess::ListShown(after_update) =
        execute_live_list(&app, &server, &["get", &board.board_id, &created.list_id])
            .await
            .expect("the updated list must remain readable")
    else {
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
    let CommandSuccess::ListShown(truncated_title_list) =
        execute_live_list(&app, &server, &["get", &board.board_id, &created.list_id])
            .await
            .expect("the list with Wekan's truncated title must remain readable")
    else {
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

    let CommandSuccess::ListShown(sanitized_wip) =
        execute_live_list(&app, &server, &["get", &board.board_id, &created.list_id])
            .await
            .expect("Wekan must strip unsupported WIP fields before returning the list")
    else {
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
        &["delete", &board.board_id, &created.list_id, "--yes"],
    )
    .await
    .expect("soft deletion must succeed") else {
        panic!("expected delete-list output")
    };
    assert_eq!(deleted.board_id, board.board_id);
    assert_eq!(deleted.list_id, created.list_id);
    assert!(deleted.deleted);
    assert_eq!(deleted.delete_mode, ListDeleteMode::Soft);

    let CommandSuccess::ListShown(soft_deleted) =
        execute_live_list(&app, &server, &["get", &board.board_id, &created.list_id])
            .await
            .expect("Wekan must retain the soft-deleted list document")
    else {
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
            &board.board_id,
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
        let CommandSuccess::CardCollection(cards) =
            execute_live_card(role_app, &server, &["list", &board.board_id, &list.list_id])
                .await
                .unwrap_or_else(|error| {
                    panic!("{role} member must be able to list cards: {error:?}")
                })
        else {
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
            &["get", &board.board_id, &list.list_id, &parent.card_id],
        )
        .await
        .unwrap_or_else(|error| panic!("{role} member must be able to get a card: {error:?}")) else {
            panic!("expected {role} card detail output")
        };
        assert_eq!(card.card_id, parent.card_id);
    }

    for (role, role_app) in [("comment-only", &comment_app), ("worker", &worker_app)] {
        let list_error =
            execute_live_card(role_app, &server, &["list", &board.board_id, &list.list_id])
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
            &["get", &board.board_id, &list.list_id, &parent.card_id],
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
            &board.board_id,
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
            &board.board_id,
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
            &board.board_id,
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
                &board.board_id,
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
                &board.board_id,
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
                &board.board_id,
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
                &board.board_id,
                &list.list_id,
                "--title",
                "Read-only create must fail",
                "--swimlane-id",
                &board.default_swimlane_id,
            ],
            "update" => vec![
                "update",
                &board.board_id,
                &list.list_id,
                &parent.card_id,
                "--title",
                "Read-only update must fail",
            ],
            "delete" => vec![
                "delete",
                &board.board_id,
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
            &board.board_id,
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

    let CommandSuccess::CardCollection(collection) =
        execute_live_card(&app, &server, &["list", &board.board_id, &list.list_id])
            .await
            .expect("the created card must be visible in its list")
    else {
        panic!("expected card collection output")
    };
    assert!(collection.cards.iter().any(|card| {
        card.card_id == created.card_id && card.title.as_deref() == Some(initial_title.as_str())
    }));

    let CommandSuccess::CardShown(initial) = execute_live_card(
        &app,
        &server,
        &["get", &board.board_id, &list.list_id, &created.card_id],
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
            &board.board_id,
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
        &["get", &board.board_id, &list.list_id, &created.card_id],
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
            &board.board_id,
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
        &["get", &board.board_id, &list.list_id, &created.card_id],
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
            &board.board_id,
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
        &["get", &board.board_id, &list.list_id, &created.card_id],
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
            &board.board_id,
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
        &["get", &board.board_id, &list.list_id, &created.card_id],
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
            &board.board_id,
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
        &["get", &board.board_id, &list.list_id, &created.card_id],
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
            &board.board_id,
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

async fn execute_live_api_output(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
    format: OutputFormat,
) -> Vec<u8> {
    let (exit, stdout, stderr) = execute_live_api_result(app, server, arguments, format).await;
    assert_eq!(
        exit,
        std::process::ExitCode::SUCCESS,
        "{}",
        String::from_utf8_lossy(&stderr)
    );
    assert!(stderr.is_empty());
    stdout
}

async fn execute_live_api_result(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
    format: OutputFormat,
) -> (std::process::ExitCode, Vec<u8>, Vec<u8>) {
    let mut argv = vec!["wekan", "--server", server, "api"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live raw API command must parse");
    let result = app
        .execute(cli.command)
        .await
        .expect("the live raw API command must reach Wekan");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = write_success(format, result, &mut stdout, &mut stderr)
        .await
        .expect("the live raw API response must render");
    (exit, stdout, stderr)
}

async fn wait_for_live_raw_checklist(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    checklist_path: &str,
    checklist_id: &str,
    title: &str,
) {
    for _ in 0..40 {
        let output = execute_live_api_output(
            app,
            server,
            &["request", "GET", checklist_path],
            OutputFormat::Json,
        )
        .await;
        let value: serde_json::Value =
            serde_json::from_slice(&output).expect("the checklist collection must be JSON");
        if value["data"]["body"]["value"]
            .as_array()
            .is_some_and(|checklists| {
                checklists.iter().any(|checklist| {
                    checklist["_id"].as_str() == Some(checklist_id)
                        && checklist["title"].as_str() == Some(title)
                })
            })
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("checklist `{checklist_id}` was not observed within 10 seconds")
}

async fn wait_for_live_raw_checklist_absent(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    checklist_path: &str,
    checklist_id: &str,
) {
    for _ in 0..40 {
        let output = execute_live_api_output(
            app,
            server,
            &["request", "GET", checklist_path],
            OutputFormat::Json,
        )
        .await;
        let value: serde_json::Value =
            serde_json::from_slice(&output).expect("the checklist collection must be JSON");
        if value["data"]["body"]["value"]
            .as_array()
            .is_some_and(|checklists| {
                checklists
                    .iter()
                    .all(|checklist| checklist["_id"].as_str() != Some(checklist_id))
            })
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("deleted checklist `{checklist_id}` remained visible for 10 seconds")
}

async fn execute_live_user(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output=json", "user"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live user command must parse");
    app.execute(cli.command).await
}

async fn register_live_board_role(
    server: &str,
    canonical_server: &str,
    admin_token: &str,
    board_id: &str,
    nonce: u128,
    role: &str,
) -> (
    TestDirectory,
    App<CapturingStore, FixedPassword, FileProfileStore>,
    String,
) {
    let username = format!("wekan_cli_card_{role}_{nonce}");
    let email = format!("{username}@example.test");
    let password = format!("Wekan-card-{role}-{nonce}!");
    let directory = TestDirectory::new(&format!("card-role-{role}"));
    let app = App::with_profile_store(
        ServerSelection::new(Some(server.to_owned()), Some("default".to_owned()), false),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password)),
        FileProfileStore::at(directory.path().to_owned()),
    );
    let registration = Cli::try_parse_from([
        "wekan",
        "--server",
        server,
        "auth",
        "register",
        "--username",
        &username,
        "--email",
        &email,
        "--password-stdin",
    ])
    .expect("the card-role registration command must parse");
    let CommandSuccess::Registration(registration) = app
        .execute(registration.command)
        .await
        .unwrap_or_else(|error| panic!("the {role} card-test account must register: {error:?}"))
    else {
        panic!("expected {role} registration output")
    };
    let user_id = registration.user_id;
    let membership = live_api_json(
        &reqwest::Client::new(),
        canonical_server,
        admin_token,
        reqwest::Method::POST,
        &format!("api/boards/{board_id}/members/{user_id}/add"),
        Some(serde_json::json!({
            "action": "add",
            "role": role
        })),
    )
    .await;
    assert!(
        membership.as_array().is_some_and(|boards| {
            boards
                .iter()
                .any(|summary| summary["_id"].as_str() == Some(board_id))
        }),
        "the board-admin request must add the {role} card-test member"
    );
    wait_for_live_board_membership(&app, server, board_id).await;
    (directory, app, user_id)
}

async fn execute_live_board(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output=json", "board"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live board command must parse");
    app.execute(cli.command).await
}

async fn execute_live_list(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output=json", "list"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live list command must parse");
    app.execute(cli.command).await
}

async fn execute_live_card(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output=json", "card"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live card command must parse");
    app.execute(cli.command).await
}

async fn execute_live_swimlane(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    arguments: &[&str],
) -> Result<CommandSuccess, AppError> {
    let mut argv = vec!["wekan", "--server", server, "--output=json", "swimlane"];
    argv.extend_from_slice(arguments);
    let cli = Cli::try_parse_from(argv).expect("the live swimlane command must parse");
    app.execute(cli.command).await
}

async fn wait_for_live_board_membership(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board_id: &str,
) {
    for _ in 0..40 {
        let result = execute_live_user(app, server, &["current"])
            .await
            .expect("the ordinary member must remain readable while polling membership");
        let CommandSuccess::UserCurrent(user) = result else {
            panic!("expected current-user output")
        };
        if user
            .boards
            .iter()
            .any(|board| board.board_id == board_id && board.is_active == Some(true))
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("ordinary member was not added to board `{board_id}` within 10 seconds")
}

async fn wait_for_live_swimlane_absent(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board_id: &str,
    swimlane_id: &str,
) {
    for _ in 0..40 {
        match execute_live_swimlane(app, server, &["get", board_id, swimlane_id]).await {
            Err(error) if error.code() == ErrorCode::NotFound => {
                assert_eq!(error.details().http_status, Some(200));
                assert_eq!(error.details().wekan_status_code, Some(404));
                return;
            }
            Ok(CommandSuccess::SwimlaneShown(_)) => {}
            Ok(other) => panic!("unexpected swimlane absence probe result: {other:?}"),
            Err(error) => panic!("unexpected swimlane absence probe error: {error:?}"),
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("deleted swimlane `{swimlane_id}` remained retrievable for 10 seconds")
}

async fn wait_for_live_swimlane_absent_from_collection(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board_id: &str,
    swimlane_id: &str,
) {
    for _ in 0..40 {
        let result = execute_live_swimlane(app, server, &["list", board_id])
            .await
            .expect("the post-delete swimlane collection must remain readable");
        let CommandSuccess::SwimlaneCollection(collection) = result else {
            panic!("expected swimlane collection output")
        };
        if collection
            .swimlanes
            .iter()
            .all(|swimlane| swimlane.swimlane_id != swimlane_id)
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("deleted swimlane `{swimlane_id}` remained in its collection for 10 seconds")
}

async fn wait_for_live_list_absent(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board_id: &str,
    list_id: &str,
) {
    for _ in 0..40 {
        match execute_live_list(app, server, &["get", board_id, list_id]).await {
            Err(error) if error.code() == ErrorCode::NotFound => return,
            Ok(CommandSuccess::ListShown(_)) => {}
            Ok(other) => panic!("unexpected list absence probe result: {other:?}"),
            Err(error) => panic!("unexpected list absence probe error: {error:?}"),
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("cascaded list `{list_id}` remained retrievable for 10 seconds")
}

async fn wait_for_live_card_absent(
    client: &reqwest::Client,
    canonical_server: &str,
    token: &str,
    board_id: &str,
    list_id: &str,
    card_id: &str,
) {
    let card_url = url::Url::parse(canonical_server)
        .expect("the canonical server URL must parse")
        .join(&format!(
            "api/boards/{board_id}/lists/{list_id}/cards/{card_id}"
        ))
        .expect("the cascade-card API path must join");
    for _ in 0..40 {
        let response = client
            .get(card_url.clone())
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .expect("the cascade-card probe must complete");
        assert!(response.status().is_success());
        let body = response
            .bytes()
            .await
            .expect("the cascade-card response body must be readable");
        if body.is_empty() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("cascaded card `{card_id}` remained retrievable for 10 seconds")
}

async fn wait_for_live_user(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    username: &str,
) -> String {
    for _ in 0..40 {
        let result = execute_live_user(app, server, &["list"])
            .await
            .expect("site admin must be able to poll the user list");
        let CommandSuccess::UserList(users) = result else {
            panic!("expected user-list output")
        };
        if let Some(user) = users
            .users
            .iter()
            .find(|user| user.username.as_deref() == Some(username))
        {
            return user.user_id.clone();
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("asynchronous creation of user `{username}` was not observed within 10 seconds")
}

async fn wait_for_live_user_absent(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    user_id: &str,
) {
    for _ in 0..40 {
        let result = execute_live_user(app, server, &["list"])
            .await
            .expect("site admin must be able to poll the user list after deletion");
        let CommandSuccess::UserList(users) = result else {
            panic!("expected user-list output")
        };
        if !users.users.iter().any(|user| user.user_id == user_id) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("deleted user `{user_id}` remained visible for 10 seconds")
}

async fn wait_for_live_board_admin_membership(
    app: &App<CapturingStore, FixedPassword, FileProfileStore>,
    server: &str,
    board_id: &str,
) {
    for _ in 0..40 {
        let result = execute_live_user(app, server, &["current"])
            .await
            .expect("the current user must remain readable while polling ownership");
        let CommandSuccess::UserCurrent(user) = result else {
            panic!("expected current-user output")
        };
        if user
            .boards
            .iter()
            .any(|board| board.board_id == board_id && board.is_admin == Some(true))
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!(
        "asynchronous ownership change for board `{board_id}` was not observed within 10 seconds"
    )
}

async fn wait_for_live_board_absent(
    client: &reqwest::Client,
    canonical_server: &str,
    token: &str,
    board_id: &str,
) {
    let url = url::Url::parse(canonical_server)
        .expect("the canonical server URL must parse")
        .join(&format!("api/boards/{board_id}"))
        .expect("the live board path must join");
    for _ in 0..40 {
        let response = client
            .get(url.clone())
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .expect("the deleted-board probe must complete");
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return;
        }
        let status = response.status();
        let body = response
            .text()
            .await
            .expect("the deleted-board probe body must be readable");
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body)
            && value.get("statusCode").and_then(serde_json::Value::as_u64) == Some(404)
        {
            return;
        }
        assert!(
            status.is_success(),
            "deleted-board probe returned unexpected HTTP {status}: {body}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    panic!("deleted board `{board_id}` remained retrievable for 10 seconds")
}

async fn live_api_json(
    client: &reqwest::Client,
    canonical_server: &str,
    token: &str,
    method: reqwest::Method,
    path: &str,
    body: Option<serde_json::Value>,
) -> serde_json::Value {
    let url = url::Url::parse(canonical_server)
        .expect("the canonical server URL must parse")
        .join(path)
        .expect("the live API path must join");
    let mut request = client
        .request(method, url)
        .header(AUTHORIZATION, format!("Bearer {token}"));
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request
        .send()
        .await
        .expect("the live setup or cleanup request must complete");
    assert!(response.status().is_success());
    let value: serde_json::Value = response
        .json()
        .await
        .expect("the live API response must be JSON");
    assert!(
        value.get("statusCode").is_none(),
        "Wekan returned an embedded error: {value}"
    );
    value
}

fn required_id(value: &serde_json::Value, resource: &str) -> String {
    value["_id"]
        .as_str()
        .unwrap_or_else(|| panic!("{resource} response must contain a string _id: {value}"))
        .to_owned()
}

#[tokio::test]
#[ignore = "requires a user-started Wekan v11.06 stack and WEKAN_E2E_URL"]
async fn named_profile_authentication_flow_matches_wekan_v11_06() {
    const PROFILE_NAME: &str = "live-e2e";

    let server = env::var("WEKAN_E2E_URL")
        .expect("set WEKAN_E2E_URL to the user-started Wekan v11.06 server URL");
    let canonical_server = ServerUrl::parse(&server)
        .expect("the live-test Wekan URL must be valid")
        .as_str()
        .to_owned();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock must be after the Unix epoch")
        .as_millis();
    let username = format!("wekan_cli_profile_e2e_{}_{}", std::process::id(), nonce);
    let email = format!("{username}@example.test");
    let password = format!("Wekan-profile-e2e-{nonce}!");

    let directory = TestDirectory::new("named-profile-auth");
    let profile_store = FileProfileStore::at(directory.path().to_owned());
    let mut profiles = ProfileDocument::default();
    profiles.insert(
        PROFILE_NAME.to_owned(),
        Profile::new(canonical_server.clone()),
    );
    profiles.set_active_profile(Some(PROFILE_NAME.to_owned()));
    profile_store
        .lock_mutation()
        .expect("the E2E profile store must be writable")
        .save(profiles)
        .expect("the named E2E profile must be persisted");

    let register_cli = Cli::try_parse_from([
        "wekan",
        "--profile",
        PROFILE_NAME,
        "--output=json",
        "auth",
        "register",
        "--username",
        &username,
        "--email",
        &email,
        "--password-stdin",
    ])
    .expect("the named-profile registration command must parse");
    let app = App::with_profile_store(
        ServerSelection::new(
            register_cli.server,
            register_cli.profile_name,
            register_cli.allow_insecure_http,
        ),
        CapturingStore::default(),
        FixedPassword(SecretString::from(password)),
        profile_store,
    );

    let registration = app
        .execute(register_cli.command)
        .await
        .expect("registration through a named profile must succeed against Wekan v11.06");
    let CommandSuccess::Registration(registration) = registration else {
        panic!("expected registration output")
    };
    assert_eq!(registration.server, canonical_server);
    assert_eq!(registration.profile, PROFILE_NAME);
    assert!(registration.credential_stored);

    let (stored_account, stored_server, stored_user_id, stored_token, _) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .expect("profile registration must save a credential");
    assert_ne!(stored_account, canonical_server);
    assert_eq!(stored_server, canonical_server);
    assert_eq!(stored_user_id, registration.user_id);

    let status_cli = Cli::try_parse_from([
        "wekan",
        "--profile",
        PROFILE_NAME,
        "--output=json",
        "auth",
        "status",
    ])
    .expect("the named-profile status command must parse");
    let status = app
        .execute(status_cli.command)
        .await
        .expect("status through a named profile must validate the stored session");
    let CommandSuccess::AuthStatus(status) = status else {
        panic!("expected authentication status output")
    };
    assert_eq!(status.server, canonical_server);
    assert_eq!(status.profile, PROFILE_NAME);
    assert_eq!(status.user.user_id, registration.user_id);

    let logout_cli = Cli::try_parse_from([
        "wekan",
        "--profile",
        PROFILE_NAME,
        "--output=json",
        "auth",
        "logout",
        "--yes",
    ])
    .expect("the named-profile logout command must parse");
    let logout = app
        .execute(logout_cli.command)
        .await
        .expect("remote logout through a named profile must succeed");
    let CommandSuccess::Logout(logout) = logout else {
        panic!("expected logout output")
    };
    assert_eq!(logout.server, canonical_server);
    assert_eq!(logout.profile, PROFILE_NAME);
    assert_eq!(logout.logout_scope, LogoutScope::CurrentToken);
    assert!(logout.remote_logout_completed);
    assert!(logout.local_credential_removed);
    assert!(!logout.credential_stored);
    assert!(app.credential_store().credential.lock().unwrap().is_none());
    assert_token_is_rejected(&canonical_server, &stored_token).await;
}

async fn assert_token_authenticates(canonical_server: &str, user_id: &str, token: &str) {
    let current_user_url = url::Url::parse(canonical_server)
        .expect("the canonical server URL must parse")
        .join("api/user")
        .expect("the authenticated-user endpoint must join");
    let response = reqwest::Client::new()
        .get(current_user_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .expect("the authenticated request must complete");

    assert!(
        response.status().is_success(),
        "the registration token must authenticate"
    );
    let user: serde_json::Value = response
        .json()
        .await
        .expect("the current-user response must be JSON");
    let returned_id = user
        .get("_id")
        .or_else(|| user.get("id"))
        .and_then(serde_json::Value::as_str);
    assert_eq!(returned_id, Some(user_id));
}

async fn assert_token_is_rejected(canonical_server: &str, token: &str) {
    let current_user_url = url::Url::parse(canonical_server)
        .expect("the canonical server URL must parse")
        .join("api/user")
        .expect("the authenticated-user endpoint must join");
    let response = reqwest::Client::new()
        .get(current_user_url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .expect("the rejected-token request must complete");

    assert!(response.status().is_success());
    let body: serde_json::Value = response
        .json()
        .await
        .expect("the rejected-token response must be JSON");
    assert_eq!(body["statusCode"], 401);
}

#[derive(Debug)]
struct ProcessOutput {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

fn spawn_production_cli(
    config_directory: &Path,
    server: &str,
    args: &[&str],
    stdin: Option<&str>,
) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wekan"));
    command
        .env("WEKAN_CONFIG_DIR", config_directory)
        .env_remove("WEKAN_PROFILE")
        .env_remove("WEKAN_URL")
        .args(["--server", server, "--output=json", "auth"])
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if args.first() == Some(&"logout") && !args.contains(&"--yes") {
        command.arg("--yes");
    }
    let mut child = command.spawn().expect("the CLI child process must start");
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .expect("piped stdin must exist")
            .write_all(input.as_bytes())
            .expect("the CLI test input must be written");
    }
    child
}

fn finish_production_cli(child: Child) -> ProcessOutput {
    let output = child
        .wait_with_output()
        .expect("the CLI child process must finish");
    ProcessOutput {
        status: output.status,
        stdout: String::from_utf8(output.stdout).expect("CLI stdout must be UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("CLI stderr must be UTF-8"),
    }
}

fn run_production_cli(
    config_directory: &Path,
    server: &str,
    args: &[&str],
    stdin: Option<&str>,
) -> ProcessOutput {
    finish_production_cli(spawn_production_cli(config_directory, server, args, stdin))
}

fn success_json(output: &ProcessOutput) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_str(&output.stdout).expect("success stdout must be JSON");
    assert_eq!(value["ok"], true, "{output:?}");
    value
}

fn error_json(output: &ProcessOutput, exit_code: i32, code: &str) -> serde_json::Value {
    assert_eq!(output.status.code(), Some(exit_code), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let value: serde_json::Value =
        serde_json::from_str(&output.stderr).expect("error stderr must be JSON");
    assert_eq!(value["ok"], false, "{output:?}");
    assert_eq!(value["error"]["code"], code, "{output:?}");
    value
}

fn assert_secret_absent(output: &ProcessOutput, secret: &str) {
    assert!(!output.stdout.contains(secret), "secret leaked on stdout");
    assert!(!output.stderr.contains(secret), "secret leaked on stderr");
}

async fn token_state(server: &str, token: &str) -> (u16, serde_json::Value) {
    let response = reqwest::Client::new()
        .get(format!("{server}api/user"))
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .await
        .expect("the token probe must complete");
    let status = response.status().as_u16();
    let body = response
        .json()
        .await
        .expect("the token probe response must be JSON");
    (status, body)
}

struct NativeCredentialCleanup {
    config_directory: PathBuf,
    server: String,
}

impl Drop for NativeCredentialCleanup {
    fn drop(&mut self) {
        let _ = run_production_cli(
            &self.config_directory,
            &self.server,
            &["logout", "--local-only"],
            None,
        );
    }
}

#[tokio::test]
#[ignore = "destructive: requires a dedicated Wekan v11.06 stack and WEKAN_OS_E2E_URL"]
async fn os_backed_cross_process_auth_mutations_are_serialized_and_hardened() {
    const PARALLELISM: usize = 24;
    const RACE_ROUNDS: usize = 12;

    let server = env::var("WEKAN_OS_E2E_URL")
        .expect("set WEKAN_OS_E2E_URL to a dedicated Wekan v11.06 server URL");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock must be after the Unix epoch")
        .as_millis();
    let username = format!("wekan_cli_os_e2e_{}_{}", std::process::id(), nonce);
    let email = format!("{username}@example.test");
    let password = format!("Wekan-os-e2e-{nonce}!");
    let password_input = format!("{password}\n");
    let store = KeyringCredentialStore;
    let config_directory = TestDirectory::new("os-backed-auth");
    let profile_store = FileProfileStore::at(config_directory.path().to_owned());

    let probe = ServerUrl::parse(&server)
        .expect("the dedicated server URL must be valid for the CLI")
        .as_str()
        .to_owned();
    let credential_namespace = profile_store
        .credential_namespace()
        .expect("the isolated profile credential namespace must resolve");
    let credential_target =
        CredentialTarget::profile_in_store("default", &credential_namespace, probe.clone());
    assert!(
        store
            .load(&credential_target)
            .expect("the native credential store must be readable")
            .is_none(),
        "the dedicated OS E2E server already has a credential; use a fresh stack/port"
    );
    let _cleanup = NativeCredentialCleanup {
        config_directory: config_directory.path().to_owned(),
        server: server.clone(),
    };

    let registration_args = [
        "register",
        "--username",
        &username,
        "--email",
        &email,
        "--password-stdin",
    ];
    let registrations: Vec<_> = (0..4)
        .map(|_| {
            spawn_production_cli(
                config_directory.path(),
                &server,
                &registration_args,
                Some(&password_input),
            )
        })
        .collect();
    let registrations: Vec<_> = registrations
        .into_iter()
        .map(finish_production_cli)
        .collect();
    for output in &registrations {
        assert_secret_absent(output, &password);
    }
    let successful_registrations: Vec<_> = registrations
        .iter()
        .filter(|output| output.status.success())
        .collect();
    assert_eq!(successful_registrations.len(), 1, "{registrations:#?}");
    let registration = success_json(successful_registrations[0]);
    let canonical_server = registration["data"]["server"]
        .as_str()
        .expect("registration must report the canonical server")
        .to_owned();
    assert_eq!(registration["data"]["credential_stored"], true);
    assert_eq!(registration["data"]["profile"], "default");
    assert_eq!(registration["data"]["profile_created"], true);
    assert_eq!(registration["data"]["profile_active"], true);
    for output in registrations
        .iter()
        .filter(|output| !output.status.success())
    {
        let error = error_json(output, 3, "credential_already_exists");
        assert_eq!(error["error"]["details"]["profile"], "default");
        assert_eq!(error["error"]["details"]["account_created"], false);
        assert_eq!(error["error"]["details"]["credential_stored"], true);
    }

    let registration_record = store
        .load(&credential_target)
        .expect("registration credential must be readable")
        .expect("registration credential must be stored");
    let registration_token = registration_record.token().expose_secret().to_owned();
    let (http_status, body) = token_state(&canonical_server, &registration_token).await;
    assert_eq!(http_status, 200);
    assert_eq!(body["_id"], registration_record.user_id());

    let local_only = run_production_cli(
        config_directory.path(),
        &server,
        &["logout", "--local-only"],
        None,
    );
    let local_only = success_json(&local_only);
    assert_eq!(local_only["data"]["remote_logout_completed"], false);
    assert_eq!(local_only["data"]["local_credential_removed"], true);
    assert!(store.load(&credential_target).unwrap().is_none());
    let (http_status, body) = token_state(&canonical_server, &registration_token).await;
    assert_eq!(http_status, 200);
    assert_eq!(body["_id"], registration_record.user_id());

    let idempotent = run_production_cli(
        config_directory.path(),
        &server,
        &["logout", "--local-only"],
        None,
    );
    let idempotent = success_json(&idempotent);
    assert_eq!(idempotent["data"]["local_credential_removed"], false);

    let missing_status = run_production_cli(config_directory.path(), &server, &["status"], None);
    let missing_status = error_json(&missing_status, 5, "credential_not_found");
    println!(
        "missing status response details: {}",
        missing_status["error"]["details"]
    );

    let missing_logout = run_production_cli(config_directory.path(), &server, &["logout"], None);
    let missing_logout = error_json(&missing_logout, 5, "credential_not_found");
    assert_eq!(
        missing_logout["error"]["details"]["remote_logout_completed"],
        false
    );
    assert_eq!(
        missing_logout["error"]["details"]["credential_stored"],
        false
    );

    let empty_password = run_production_cli(
        config_directory.path(),
        &server,
        &["login", "--username", &username, "--password-stdin"],
        Some("\n"),
    );
    error_json(&empty_password, 2, "invalid_input");

    let unused_listener =
        TcpListener::bind("127.0.0.1:0").expect("an unused loopback port must be reservable");
    let unused_port = unused_listener
        .local_addr()
        .expect("the unused listener must have an address")
        .port();
    drop(unused_listener);
    let unreachable_server = format!("http://127.0.0.1:{unused_port}");
    let transport_config_directory = TestDirectory::new("os-backed-transport-failure");
    let transport_failure = run_production_cli(
        transport_config_directory.path(),
        &unreachable_server,
        &["login", "--username", &username, "--password-stdin"],
        Some(&password_input),
    );
    assert_secret_absent(&transport_failure, &password);
    let transport_failure = error_json(&transport_failure, 4, "transport_error");
    assert_eq!(
        transport_failure["error"]["details"]["outcome_unknown"],
        true
    );
    assert!(
        FileProfileStore::at(transport_config_directory.path().to_owned())
            .read()
            .unwrap()
            .document()
            .is_empty()
    );

    let wrong_password = format!("wrong-{password}");
    let wrong_password_input = format!("{wrong_password}\n");
    let rejected = run_production_cli(
        config_directory.path(),
        &server,
        &["login", "--username", &username, "--password-stdin"],
        Some(&wrong_password_input),
    );
    assert_secret_absent(&rejected, &wrong_password);
    let rejected = error_json(&rejected, 5, "login_rejected");
    assert_eq!(rejected["error"]["details"]["http_status"], 401);

    let login_args = ["login", "--username", &username, "--password-stdin"];
    let login = run_production_cli(
        config_directory.path(),
        &server,
        &login_args,
        Some(&password_input),
    );
    assert_secret_absent(&login, &password);
    success_json(&login);
    let current_record = store.load(&credential_target).unwrap().unwrap();
    let current_token = current_record.token().expose_secret().to_owned();

    let remote_logout = run_production_cli(config_directory.path(), &server, &["logout"], None);
    let remote_logout = success_json(&remote_logout);
    assert_eq!(remote_logout["data"]["logout_scope"], "current_token");
    assert_eq!(remote_logout["data"]["remote_logout_completed"], true);
    assert_eq!(remote_logout["data"]["local_credential_removed"], true);
    assert!(store.load(&credential_target).unwrap().is_none());
    let (current_http, current_body) = token_state(&canonical_server, &current_token).await;
    assert_eq!(current_http, 200);
    assert_eq!(current_body["statusCode"], 401);
    let (older_http, older_body) = token_state(&canonical_server, &registration_token).await;
    assert_eq!(older_http, 200);
    assert_eq!(older_body["_id"], registration_record.user_id());

    success_json(&run_production_cli(
        config_directory.path(),
        &server,
        &login_args,
        Some(&password_input),
    ));
    let cli_all_token = store
        .load(&credential_target)
        .unwrap()
        .unwrap()
        .token()
        .expose_secret()
        .to_owned();
    let direct_login = reqwest::Client::new()
        .post(format!("{canonical_server}users/login"))
        .json(&serde_json::json!({"username": username, "password": password}))
        .send()
        .await
        .expect("the independent login must complete");
    let direct_status = direct_login.status().as_u16();
    let direct_body: serde_json::Value = direct_login
        .json()
        .await
        .expect("the independent login response must be JSON");
    assert_eq!(direct_status, 200);
    let independent_token = direct_body["token"]
        .as_str()
        .expect("the independent login must return a token")
        .to_owned();
    let all_logout =
        run_production_cli(config_directory.path(), &server, &["logout", "--all"], None);
    let all_logout = success_json(&all_logout);
    assert_eq!(all_logout["data"]["logout_scope"], "all_tokens");
    assert_eq!(all_logout["data"]["remote_logout_completed"], true);
    for token in [&registration_token, &cli_all_token, &independent_token] {
        let (status, body) = token_state(&canonical_server, token).await;
        assert_eq!(status, 200);
        assert_eq!(body["statusCode"], 401);
    }
    println!(
        "live status behavior: login=200, authenticated user=200, revoked token=200 with embedded statusCode=401"
    );

    success_json(&run_production_cli(
        config_directory.path(),
        &server,
        &login_args,
        Some(&password_input),
    ));
    let local_only_token = store
        .load(&credential_target)
        .unwrap()
        .unwrap()
        .token()
        .expose_secret()
        .to_owned();
    success_json(&run_production_cli(
        config_directory.path(),
        &server,
        &["logout", "--local-only"],
        None,
    ));
    let (status, body) = token_state(&canonical_server, &local_only_token).await;
    assert_eq!(status, 200);
    assert_eq!(body["_id"], registration_record.user_id());

    let expired_record = CredentialRecord::new(
        canonical_server.clone(),
        registration_record.user_id().to_owned(),
        SecretString::from("expired-os-backed-token".to_owned()),
        OffsetDateTime::now_utc() - Duration::hours(1),
    );
    assert_eq!(
        store.create(&credential_target, &expired_record).unwrap(),
        CredentialCreateOutcome::Created
    );
    let expired_login = run_production_cli(
        config_directory.path(),
        &server,
        &login_args,
        Some(&password_input),
    );
    error_json(&expired_login, 3, "credential_already_exists");
    success_json(&run_production_cli(
        config_directory.path(),
        &server,
        &["logout", "--local-only"],
        None,
    ));

    let invalid_record = CredentialRecord::new(
        canonical_server.clone(),
        registration_record.user_id().to_owned(),
        SecretString::from("definitely-invalid-os-backed-token".to_owned()),
        OffsetDateTime::now_utc() + Duration::hours(1),
    );
    assert_eq!(
        store.create(&credential_target, &invalid_record).unwrap(),
        CredentialCreateOutcome::Created
    );
    let occupied_login = run_production_cli(
        config_directory.path(),
        &server,
        &login_args,
        Some(&password_input),
    );
    error_json(&occupied_login, 3, "credential_already_exists");
    let invalid_logout = run_production_cli(config_directory.path(), &server, &["logout"], None);
    let invalid_logout = error_json(&invalid_logout, 5, "authentication_rejected");
    assert_eq!(invalid_logout["error"]["details"]["http_status"], 401);
    assert_eq!(
        invalid_logout["error"]["details"]["remote_logout_completed"],
        false
    );
    assert_eq!(
        invalid_logout["error"]["details"]["credential_stored"],
        true
    );
    assert!(store.load(&credential_target).unwrap().is_some());
    success_json(&run_production_cli(
        config_directory.path(),
        &server,
        &["logout", "--local-only"],
        None,
    ));

    keyring::Entry::new("wekan-cli", credential_target.account())
        .unwrap()
        .set_secret(b"{malformed credential")
        .unwrap();
    let corrupt_login = run_production_cli(
        config_directory.path(),
        &server,
        &login_args,
        Some(&password_input),
    );
    error_json(&corrupt_login, 3, "credential_already_exists");
    let corrupt_status = run_production_cli(config_directory.path(), &server, &["status"], None);
    let corrupt_status = error_json(&corrupt_status, 6, "credential_store_failed");
    println!(
        "corrupt status response details: {}",
        corrupt_status["error"]["details"]
    );
    success_json(&run_production_cli(
        config_directory.path(),
        &server,
        &["logout", "--local-only"],
        None,
    ));

    let parallel_logins: Vec<_> = (0..PARALLELISM)
        .map(|_| {
            spawn_production_cli(
                config_directory.path(),
                &server,
                &login_args,
                Some(&password_input),
            )
        })
        .collect();
    let parallel_logins: Vec<_> = parallel_logins
        .into_iter()
        .map(finish_production_cli)
        .collect();
    for output in &parallel_logins {
        assert_secret_absent(output, &password);
    }
    assert_eq!(
        parallel_logins
            .iter()
            .filter(|output| output.status.success())
            .count(),
        1,
        "{parallel_logins:#?}"
    );
    for output in parallel_logins
        .iter()
        .filter(|output| !output.status.success())
    {
        let error = error_json(output, 3, "credential_already_exists");
        assert_eq!(error["error"]["details"]["session_created"], false);
    }
    success_json(&run_production_cli(
        config_directory.path(),
        &server,
        &["status"],
        None,
    ));

    let parallel_local_logouts: Vec<_> = (0..PARALLELISM)
        .map(|_| {
            spawn_production_cli(
                config_directory.path(),
                &server,
                &["logout", "--local-only"],
                None,
            )
        })
        .collect();
    let removed_count = parallel_local_logouts
        .into_iter()
        .map(finish_production_cli)
        .map(|output| success_json(&output))
        .filter(|value| value["data"]["local_credential_removed"] == true)
        .count();
    assert_eq!(removed_count, 1);
    assert!(store.load(&credential_target).unwrap().is_none());

    for _ in 0..RACE_ROUNDS {
        success_json(&run_production_cli(
            config_directory.path(),
            &server,
            &["logout", "--local-only"],
            None,
        ));
        success_json(&run_production_cli(
            config_directory.path(),
            &server,
            &login_args,
            Some(&password_input),
        ));
        let login = spawn_production_cli(
            config_directory.path(),
            &server,
            &login_args,
            Some(&password_input),
        );
        let logout = spawn_production_cli(config_directory.path(), &server, &["logout"], None);
        let login = finish_production_cli(login);
        if login.status.success() {
            success_json(&login);
        } else {
            let error = error_json(&login, 3, "credential_already_exists");
            assert_eq!(error["error"]["details"]["session_created"], false);
        }
        success_json(&finish_production_cli(logout));
        let status = run_production_cli(config_directory.path(), &server, &["status"], None);
        if status.status.success() {
            let status = success_json(&status);
            assert_eq!(status["data"]["authenticated"], true);
        } else {
            error_json(&status, 5, "credential_not_found");
        }
    }

    for _ in 0..RACE_ROUNDS {
        success_json(&run_production_cli(
            config_directory.path(),
            &server,
            &["logout", "--local-only"],
            None,
        ));
        success_json(&run_production_cli(
            config_directory.path(),
            &server,
            &login_args,
            Some(&password_input),
        ));
        let login = spawn_production_cli(
            config_directory.path(),
            &server,
            &login_args,
            Some(&password_input),
        );
        let logout = spawn_production_cli(
            config_directory.path(),
            &server,
            &["logout", "--local-only"],
            None,
        );
        let login = finish_production_cli(login);
        if login.status.success() {
            success_json(&login);
        } else {
            let error = error_json(&login, 3, "credential_already_exists");
            assert_eq!(error["error"]["details"]["session_created"], false);
        }
        success_json(&finish_production_cli(logout));
        let status = run_production_cli(config_directory.path(), &server, &["status"], None);
        if status.status.success() {
            let status = success_json(&status);
            assert_eq!(status["data"]["authenticated"], true);
        } else {
            error_json(&status, 5, "credential_not_found");
        }
    }

    success_json(&run_production_cli(
        config_directory.path(),
        &server,
        &["logout", "--local-only"],
        None,
    ));
    success_json(&run_production_cli(
        config_directory.path(),
        &server,
        &login_args,
        Some(&password_input),
    ));
    success_json(&run_production_cli(
        config_directory.path(),
        &server,
        &["logout", "--all"],
        None,
    ));
    assert!(store.load(&credential_target).unwrap().is_none());
    println!(
        "OS-backed cross-process stress passed: {PARALLELISM} parallel logins, {PARALLELISM} parallel local logouts, {RACE_ROUNDS} login/remote-logout races, {RACE_ROUNDS} login/local-logout races"
    );
}
