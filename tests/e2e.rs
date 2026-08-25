use std::{
    env,
    io::Write,
    net::TcpListener,
    process::{Child, Command, ExitStatus, Stdio},
    sync::Mutex,
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
    command_result::{CommandSuccess, LogoutScope},
    credentials::{
        CredentialDeleteOutcome, CredentialError, CredentialRecord, CredentialStore,
        KeyringCredentialStore, LoginSecretMode, LoginSecrets, SecretInputProvider,
    },
    error::{AppError, ErrorCode},
};

#[derive(Default)]
struct CapturingStore {
    credential: Mutex<Option<(String, String, String, OffsetDateTime)>>,
}

impl CredentialStore for CapturingStore {
    fn check_available(&self, _account: &str) -> Result<(), CredentialError> {
        Ok(())
    }

    fn load(&self, account: &str) -> Result<Option<CredentialRecord>, CredentialError> {
        Ok(self
            .credential
            .lock()
            .unwrap()
            .as_ref()
            .filter(|(stored_account, ..)| stored_account == account)
            .map(|(server, user_id, token, token_expires)| {
                CredentialRecord::new(
                    server.clone(),
                    user_id.clone(),
                    SecretString::from(token.clone()),
                    *token_expires,
                )
            }))
    }

    fn save(&self, account: &str, record: &CredentialRecord) -> Result<(), CredentialError> {
        *self.credential.lock().unwrap() = Some((
            account.to_owned(),
            record.user_id().to_owned(),
            record.token().expose_secret().to_owned(),
            record.token_expires(),
        ));
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<bool, CredentialError> {
        let mut credential = self.credential.lock().unwrap();
        if credential
            .as_ref()
            .is_some_and(|(stored_account, ..)| stored_account == account)
        {
            credential.take();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn delete_if_matches(
        &self,
        account: &str,
        expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        let mut credential = self.credential.lock().unwrap();
        let Some((server, user_id, token, token_expires)) = credential.as_ref() else {
            return Ok(CredentialDeleteOutcome::Absent);
        };
        if server != account {
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
    fn read_registration_password(&self, _from_stdin: bool) -> Result<SecretString, AppError> {
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
    let app = App::new(cli.server, cli.allow_insecure_http, store, passwords);

    app.execute(cli.command)
        .await
        .expect("registration must succeed against Wekan v11.06");

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

    let (canonical_server, user_id, token, _) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .expect("registration must save the credential");
    assert_token_authenticates(&canonical_server, &user_id, &token).await;

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
    let (username_server, username_user_id, username_token, _) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .expect("username login must replace the stored credential");
    assert_eq!(username_user_id, user_id);
    assert_token_authenticates(&username_server, &username_user_id, &username_token).await;

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

    let (email_server, email_user_id, email_token, _) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .as_ref()
        .cloned()
        .expect("email login must replace the stored credential");
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
    let (_, _, current_token, _) = app
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
    let (_, _, all_token, _) = app
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
    let rejected_app = App::new(
        rejected_cli.server,
        rejected_cli.allow_insecure_http,
        CapturingStore::default(),
        FixedPassword(SecretString::from("definitely-wrong-password".to_owned())),
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

fn spawn_production_cli(server: &str, args: &[&str], stdin: Option<&str>) -> Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wekan"));
    command
        .args(["--server", server, "--output=json", "auth"])
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
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

fn run_production_cli(server: &str, args: &[&str], stdin: Option<&str>) -> ProcessOutput {
    finish_production_cli(spawn_production_cli(server, args, stdin))
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

struct NativeCredentialCleanup(String);

impl Drop for NativeCredentialCleanup {
    fn drop(&mut self) {
        let _ = run_production_cli(&self.0, &["logout", "--local-only"], None);
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

    let probe = ServerUrl::parse(&server, false)
        .expect("the dedicated server URL must be valid for the CLI")
        .as_str()
        .to_owned();
    assert!(
        store
            .load(&probe)
            .expect("the native credential store must be readable")
            .is_none(),
        "the dedicated OS E2E server already has a credential; use a fresh stack/port"
    );
    let _cleanup = NativeCredentialCleanup(server.clone());

    let registration_args = [
        "register",
        "--username",
        &username,
        "--email",
        &email,
        "--password-stdin",
    ];
    let registrations: Vec<_> = (0..4)
        .map(|_| spawn_production_cli(&server, &registration_args, Some(&password_input)))
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
    let duplicate_statuses: Vec<_> = registrations
        .iter()
        .filter(|output| !output.status.success())
        .map(|output| {
            let error = error_json(output, 5, "registration_rejected");
            (
                error["error"]["details"]["http_status"].as_u64(),
                error["error"]["details"]["server_error"]
                    .as_str()
                    .map(str::to_owned),
            )
        })
        .collect();
    println!("parallel duplicate registration responses: {duplicate_statuses:?}");

    let registration_record = store
        .load(&canonical_server)
        .expect("registration credential must be readable")
        .expect("registration credential must be stored");
    let registration_token = registration_record.token().expose_secret().to_owned();
    let (http_status, body) = token_state(&canonical_server, &registration_token).await;
    assert_eq!(http_status, 200);
    assert_eq!(body["_id"], registration_record.user_id());

    let local_only = run_production_cli(&server, &["logout", "--local-only"], None);
    let local_only = success_json(&local_only);
    assert_eq!(local_only["data"]["remote_logout_completed"], false);
    assert_eq!(local_only["data"]["local_credential_removed"], true);
    assert!(store.load(&canonical_server).unwrap().is_none());
    let (http_status, body) = token_state(&canonical_server, &registration_token).await;
    assert_eq!(http_status, 200);
    assert_eq!(body["_id"], registration_record.user_id());

    let idempotent = run_production_cli(&server, &["logout", "--local-only"], None);
    let idempotent = success_json(&idempotent);
    assert_eq!(idempotent["data"]["local_credential_removed"], false);

    let missing_status = run_production_cli(&server, &["status"], None);
    let missing_status = error_json(&missing_status, 5, "credential_not_found");
    println!(
        "missing status response details: {}",
        missing_status["error"]["details"]
    );

    let missing_logout = run_production_cli(&server, &["logout"], None);
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
    let transport_failure = run_production_cli(
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

    let wrong_password = format!("wrong-{password}");
    let wrong_password_input = format!("{wrong_password}\n");
    let rejected = run_production_cli(
        &server,
        &["login", "--username", &username, "--password-stdin"],
        Some(&wrong_password_input),
    );
    assert_secret_absent(&rejected, &wrong_password);
    let rejected = error_json(&rejected, 5, "login_rejected");
    assert_eq!(rejected["error"]["details"]["http_status"], 401);

    let login_args = ["login", "--username", &username, "--password-stdin"];
    let login = run_production_cli(&server, &login_args, Some(&password_input));
    assert_secret_absent(&login, &password);
    success_json(&login);
    let current_record = store.load(&canonical_server).unwrap().unwrap();
    let current_token = current_record.token().expose_secret().to_owned();

    let remote_logout = run_production_cli(&server, &["logout"], None);
    let remote_logout = success_json(&remote_logout);
    assert_eq!(remote_logout["data"]["logout_scope"], "current_token");
    assert_eq!(remote_logout["data"]["remote_logout_completed"], true);
    assert_eq!(remote_logout["data"]["local_credential_removed"], true);
    assert!(store.load(&canonical_server).unwrap().is_none());
    let (current_http, current_body) = token_state(&canonical_server, &current_token).await;
    assert_eq!(current_http, 200);
    assert_eq!(current_body["statusCode"], 401);
    let (older_http, older_body) = token_state(&canonical_server, &registration_token).await;
    assert_eq!(older_http, 200);
    assert_eq!(older_body["_id"], registration_record.user_id());

    success_json(&run_production_cli(
        &server,
        &login_args,
        Some(&password_input),
    ));
    let cli_all_token = store
        .load(&canonical_server)
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
    let all_logout = run_production_cli(&server, &["logout", "--all"], None);
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
        &server,
        &login_args,
        Some(&password_input),
    ));
    let local_only_token = store
        .load(&canonical_server)
        .unwrap()
        .unwrap()
        .token()
        .expose_secret()
        .to_owned();
    success_json(&run_production_cli(
        &server,
        &["logout", "--local-only"],
        None,
    ));
    let (status, body) = token_state(&canonical_server, &local_only_token).await;
    assert_eq!(status, 200);
    assert_eq!(body["_id"], registration_record.user_id());

    let invalid_record = CredentialRecord::new(
        canonical_server.clone(),
        registration_record.user_id().to_owned(),
        SecretString::from("definitely-invalid-os-backed-token".to_owned()),
        OffsetDateTime::now_utc() + Duration::hours(1),
    );
    store.save(&canonical_server, &invalid_record).unwrap();
    let invalid_logout = run_production_cli(&server, &["logout"], None);
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
    assert!(store.load(&canonical_server).unwrap().is_some());
    success_json(&run_production_cli(
        &server,
        &["logout", "--local-only"],
        None,
    ));

    keyring::Entry::new("wekan-cli", &canonical_server)
        .unwrap()
        .set_secret(b"{malformed credential")
        .unwrap();
    let corrupt_status = run_production_cli(&server, &["status"], None);
    let corrupt_status = error_json(&corrupt_status, 6, "credential_store_failed");
    println!(
        "corrupt status response details: {}",
        corrupt_status["error"]["details"]
    );
    success_json(&run_production_cli(
        &server,
        &["logout", "--local-only"],
        None,
    ));

    let parallel_logins: Vec<_> = (0..PARALLELISM)
        .map(|_| spawn_production_cli(&server, &login_args, Some(&password_input)))
        .collect();
    for output in parallel_logins.into_iter().map(finish_production_cli) {
        assert_secret_absent(&output, &password);
        success_json(&output);
    }
    success_json(&run_production_cli(&server, &["status"], None));

    let parallel_local_logouts: Vec<_> = (0..PARALLELISM)
        .map(|_| spawn_production_cli(&server, &["logout", "--local-only"], None))
        .collect();
    let removed_count = parallel_local_logouts
        .into_iter()
        .map(finish_production_cli)
        .map(|output| success_json(&output))
        .filter(|value| value["data"]["local_credential_removed"] == true)
        .count();
    assert_eq!(removed_count, 1);
    assert!(store.load(&canonical_server).unwrap().is_none());

    for _ in 0..RACE_ROUNDS {
        success_json(&run_production_cli(
            &server,
            &login_args,
            Some(&password_input),
        ));
        let login = spawn_production_cli(&server, &login_args, Some(&password_input));
        let logout = spawn_production_cli(&server, &["logout"], None);
        success_json(&finish_production_cli(login));
        success_json(&finish_production_cli(logout));
        let status = run_production_cli(&server, &["status"], None);
        if status.status.success() {
            let status = success_json(&status);
            assert_eq!(status["data"]["authenticated"], true);
        } else {
            error_json(&status, 5, "credential_not_found");
        }
    }

    for _ in 0..RACE_ROUNDS {
        success_json(&run_production_cli(
            &server,
            &login_args,
            Some(&password_input),
        ));
        let login = spawn_production_cli(&server, &login_args, Some(&password_input));
        let logout = spawn_production_cli(&server, &["logout", "--local-only"], None);
        success_json(&finish_production_cli(login));
        success_json(&finish_production_cli(logout));
        let status = run_production_cli(&server, &["status"], None);
        if status.status.success() {
            let status = success_json(&status);
            assert_eq!(status["data"]["authenticated"], true);
        } else {
            error_json(&status, 5, "credential_not_found");
        }
    }

    success_json(&run_production_cli(
        &server,
        &login_args,
        Some(&password_input),
    ));
    success_json(&run_production_cli(&server, &["logout", "--all"], None));
    assert!(store.load(&canonical_server).unwrap().is_none());
    println!(
        "OS-backed cross-process stress passed: {PARALLELISM} parallel logins, {PARALLELISM} parallel local logouts, {RACE_ROUNDS} login/remote-logout races, {RACE_ROUNDS} login/local-logout races"
    );
}
