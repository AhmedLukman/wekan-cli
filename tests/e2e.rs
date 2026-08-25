use std::{
    env,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use reqwest::header::AUTHORIZATION;
use secrecy::{ExposeSecret, SecretString};
use wekan_cli::{
    app::App,
    cli::Cli,
    credentials::{
        CredentialError, CredentialRecord, CredentialStore, LoginSecretMode, LoginSecrets,
        SecretInputProvider,
    },
    error::{AppError, ErrorCode},
};

#[derive(Default)]
struct CapturingStore {
    credential: Mutex<Option<(String, String, String)>>,
}

impl CredentialStore for CapturingStore {
    fn check_available(&self, _account: &str) -> Result<(), CredentialError> {
        Ok(())
    }

    fn save(&self, account: &str, record: &CredentialRecord) -> Result<(), CredentialError> {
        *self.credential.lock().unwrap() = Some((
            account.to_owned(),
            record.user_id().to_owned(),
            record.token().expose_secret().to_owned(),
        ));
        Ok(())
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

    let (canonical_server, user_id, token) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .take()
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
    let (username_server, username_user_id, username_token) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .take()
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
    let (email_server, email_user_id, email_token) = app
        .credential_store()
        .credential
        .lock()
        .unwrap()
        .take()
        .expect("email login must replace the stored credential");
    assert_eq!(email_user_id, user_id);
    assert_token_authenticates(&email_server, &email_user_id, &email_token).await;

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
