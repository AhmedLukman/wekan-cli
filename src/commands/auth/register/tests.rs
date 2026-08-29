use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

use clap::Parser;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use super::{RegisterArgs, execute as execute_target, map_client_error};
use crate::{
    cli::Cli,
    client::{ClientError, WekanClientFactory},
    command_result::CommandSuccess,
    commands::{RootCommand, auth::AuthCommand},
    config::ResolvedTarget,
    credentials::{
        CredentialCreateOutcome, CredentialError, CredentialRecord, CredentialStore,
        CredentialTarget, LoginSecretMode, LoginSecrets, SecretInputProvider,
    },
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    redaction::Redactor,
};

async fn execute(
    args: RegisterArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    let mut target = ResolvedTarget::for_test(client_factory);
    execute_target(args, &mut target, credential_store, secret_input).await
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SavedCredential {
    account: String,
    user_id: String,
    token: String,
}

#[derive(Default)]
struct FakeCredentialStore {
    available: AtomicBool,
    fail_save: AtomicBool,
    saved: Mutex<Vec<SavedCredential>>,
}

impl FakeCredentialStore {
    fn available() -> Self {
        Self {
            available: AtomicBool::new(true),
            ..Self::default()
        }
    }
}

impl CredentialStore for FakeCredentialStore {
    fn check_available(&self, _target: &CredentialTarget) -> Result<(), CredentialError> {
        if self.available.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(CredentialError::Unavailable("test unavailable".to_owned()))
        }
    }

    fn exists(&self, target: &CredentialTarget) -> Result<bool, CredentialError> {
        Ok(self
            .saved
            .lock()
            .unwrap()
            .iter()
            .any(|credential| credential.account == target.account()))
    }

    fn load(
        &self,
        _target: &CredentialTarget,
    ) -> Result<Option<CredentialRecord>, CredentialError> {
        Ok(None)
    }

    fn create(
        &self,
        target: &CredentialTarget,
        record: &CredentialRecord,
    ) -> Result<CredentialCreateOutcome, CredentialError> {
        if self.exists(target)? {
            return Ok(CredentialCreateOutcome::AlreadyExists);
        }
        if self.fail_save.load(Ordering::SeqCst) {
            return Err(CredentialError::Store(format!(
                "failed while handling {}",
                record.token().expose_secret()
            )));
        }
        let mut saved = self.saved.lock().unwrap();
        saved.push(SavedCredential {
            account: target.account().to_owned(),
            user_id: record.user_id().to_owned(),
            token: record.token().expose_secret().to_owned(),
        });
        Ok(CredentialCreateOutcome::Created)
    }

    fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
        panic!("registration must never delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<crate::credentials::CredentialDeleteOutcome, CredentialError> {
        panic!("registration must never conditionally delete credentials")
    }
}

struct FakeSecretInput {
    read: Arc<AtomicBool>,
}

impl SecretInputProvider for FakeSecretInput {
    fn read_new_account_password(&self, _from_stdin: bool) -> Result<SecretString, AppError> {
        self.read.store(true, Ordering::SeqCst);
        Ok(SecretString::from("test-password".to_owned()))
    }

    fn read_login_secrets(&self, _mode: LoginSecretMode) -> Result<LoginSecrets, AppError> {
        self.read.store(true, Ordering::SeqCst);
        Ok(LoginSecrets::new(
            SecretString::from("test-password".to_owned()),
            None,
        ))
    }
}

fn args() -> RegisterArgs {
    RegisterArgs {
        username: Some("alice".to_owned()),
        email: Some("alice@example.com".to_owned()),
        password_stdin: false,
    }
}

async fn mount_success(server: &MockServer, user_id: &str, token: &str) {
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": user_id,
            "token": token,
            "tokenExpires": "2030-01-02T03:04:05Z"
        })))
        .mount(server)
        .await;
}

#[test]
fn registration_requires_an_identity() {
    let error = Cli::try_parse_from([
        "wekan",
        "--server",
        "https://wekan.example",
        "auth",
        "register",
        "--password-stdin",
    ])
    .expect_err("registration without an identity must fail");

    let message = error.to_string();
    assert!(message.contains("--username <USERNAME>"));
    assert!(message.contains("--email <EMAIL>"));
}

#[test]
fn registration_accepts_username_and_email_together() {
    let cli = Cli::try_parse_from([
        "wekan",
        "--server",
        "https://wekan.example",
        "auth",
        "register",
        "--username",
        "alice",
        "--email",
        "alice@example.com",
        "--password-stdin",
    ])
    .expect("both identity fields should be accepted");

    let RootCommand::Auth(auth) = cli.command else {
        panic!("expected auth command")
    };
    let AuthCommand::Register(args) = auth.command else {
        panic!("expected the register command")
    };
    assert_eq!(args.username.as_deref(), Some("alice"));
    assert_eq!(args.email.as_deref(), Some("alice@example.com"));
}

#[test]
fn registration_accepts_either_identity_independently() {
    for identity in [["--username", "alice"], ["--email", "alice@example.com"]] {
        Cli::try_parse_from([
            "wekan",
            "--server",
            "https://wekan.example",
            "auth",
            "register",
            identity[0],
            identity[1],
            "--password-stdin",
        ])
        .expect("either identity should be accepted");
    }
}

#[test]
fn empty_identity_is_rejected() {
    let error = Cli::try_parse_from([
        "wekan",
        "--server",
        "https://wekan.example",
        "auth",
        "register",
        "--username",
        "",
    ])
    .expect_err("an empty identity must fail");

    assert!(error.to_string().contains("value must not be empty"));
}

#[test]
fn redacts_passwords_from_server_errors() {
    let password = SecretString::from("secret-password".to_owned());
    let redactor = Redactor::with_secret(&password);
    let error = map_client_error(
        ClientError::Server {
            status: reqwest::StatusCode::BAD_REQUEST,
            server_error: Some("bad secret-password".to_owned()),
            server_reason: Some("secret-password rejected".to_owned()),
            retry_after_seconds: None,
        },
        &redactor,
    );

    assert_eq!(error.code(), ErrorCode::RegistrationRejected);
    assert_eq!(error.exit_code(), StableExitCode::Server);
    let details = serde_json::to_string(error.details()).unwrap();
    assert!(!details.contains("secret-password"));
    assert!(details.contains("[REDACTED]"));
}

#[test]
fn registration_errors_preserve_mutation_uncertainty() {
    let password = SecretString::from("secret-password".to_owned());
    let redactor = Redactor::with_secret(&password);
    let error = map_client_error(
        ClientError::Server {
            status: reqwest::StatusCode::BAD_REQUEST,
            server_error: Some("database failure".to_owned()),
            server_reason: Some("login token could not be inserted".to_owned()),
            retry_after_seconds: None,
        },
        &redactor,
    );

    assert_eq!(error.code(), ErrorCode::RegistrationRejected);
    assert_eq!(error.details().outcome_unknown, Some(true));
    assert!(error.message().contains("check the server before retrying"));

    let upstream_error = map_client_error(
        ClientError::Server {
            status: reqwest::StatusCode::BAD_GATEWAY,
            server_error: None,
            server_reason: None,
            retry_after_seconds: Some(29),
        },
        &redactor,
    );

    assert_eq!(upstream_error.code(), ErrorCode::ServerError);
    assert_eq!(upstream_error.exit_code(), StableExitCode::Server);
    assert_eq!(upstream_error.details().http_status, Some(502));
    assert_eq!(upstream_error.details().outcome_unknown, Some(true));
    assert_eq!(upstream_error.details().retry_after_seconds, None);
    assert!(
        upstream_error
            .message()
            .contains("check the server before retrying")
    );

    let limited_error = map_client_error(
        ClientError::Server {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            server_error: None,
            server_reason: None,
            retry_after_seconds: Some(31),
        },
        &redactor,
    );
    assert_eq!(limited_error.details().retry_after_seconds, Some(31));
}

#[test]
fn invalid_registration_success_preserves_the_http_status() {
    let password = SecretString::from("secret-password".to_owned());
    let redactor = Redactor::with_secret(&password);
    let error = map_client_error(
        ClientError::Protocol {
            message: "invalid JSON or missing fields".to_owned(),
            success_status_received: true,
        },
        &redactor,
    );

    assert_eq!(error.code(), ErrorCode::ProtocolError);
    assert_eq!(error.details().http_status, Some(200));
    assert_eq!(error.details().account_created, Some(true));
}

#[test]
fn unreadable_registration_error_bodies_preserve_status_and_mutation_uncertainty() {
    let password = SecretString::from("secret-password".to_owned());
    let redactor = Redactor::with_secret(&password);
    let error = map_client_error(
        ClientError::ResponseTooLarge {
            limit_bytes: 1024 * 1024,
            status: reqwest::StatusCode::BAD_REQUEST,
            retry_after_seconds: None,
        },
        &redactor,
    );

    assert_eq!(error.code(), ErrorCode::RegistrationRejected);
    assert_eq!(error.exit_code(), StableExitCode::Server);
    assert_eq!(error.details().http_status, Some(400));
    assert_eq!(error.details().account_created, None);
    assert_eq!(error.details().outcome_unknown, Some(true));
    assert!(error.message().contains("check the server before retrying"));

    let disabled_error = map_client_error(
        ClientError::ResponseTooLarge {
            limit_bytes: 1024 * 1024,
            status: reqwest::StatusCode::FORBIDDEN,
            retry_after_seconds: None,
        },
        &redactor,
    );
    assert_eq!(disabled_error.code(), ErrorCode::RegistrationDisabled);
    assert_eq!(disabled_error.exit_code(), StableExitCode::Server);
    assert_eq!(disabled_error.details().http_status, Some(403));
    assert_eq!(disabled_error.details().outcome_unknown, None);

    let limited_error = map_client_error(
        ClientError::ResponseTooLarge {
            limit_bytes: 1024 * 1024,
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            retry_after_seconds: Some(37),
        },
        &redactor,
    );
    assert_eq!(limited_error.details().retry_after_seconds, Some(37));

    let success_error = map_client_error(
        ClientError::ResponseTooLarge {
            limit_bytes: 1024 * 1024,
            status: reqwest::StatusCode::OK,
            retry_after_seconds: None,
        },
        &redactor,
    );
    assert_eq!(success_error.details().http_status, Some(200));
    assert_eq!(success_error.details().account_created, Some(true));
    assert_eq!(success_error.details().outcome_unknown, None);

    let upstream_error = map_client_error(
        ClientError::ResponseTooLarge {
            limit_bytes: 1024 * 1024,
            status: reqwest::StatusCode::GATEWAY_TIMEOUT,
            retry_after_seconds: None,
        },
        &redactor,
    );
    assert_eq!(upstream_error.code(), ErrorCode::ServerError);
    assert_eq!(upstream_error.exit_code(), StableExitCode::Server);
    assert_eq!(upstream_error.details().http_status, Some(504));
    assert_eq!(upstream_error.details().account_created, None);
    assert_eq!(upstream_error.details().outcome_unknown, Some(true));
    assert!(
        upstream_error
            .message()
            .contains("check the server before retrying")
    );
}

#[test]
fn redirect_errors_use_the_transport_contract() {
    let password = SecretString::from("secret-password".to_owned());
    let redactor = Redactor::with_secret(&password);
    let error = map_client_error(
        ClientError::UnexpectedRedirect {
            status: reqwest::StatusCode::TEMPORARY_REDIRECT,
        },
        &redactor,
    );

    assert_eq!(error.code(), ErrorCode::UnexpectedRedirect);
    assert_eq!(error.exit_code(), StableExitCode::Transport);
    assert_eq!(error.details().http_status, Some(307));
}

#[tokio::test]
async fn successful_registration_creates_the_token_and_returns_only_metadata() {
    let server = MockServer::start().await;
    mount_success(&server, "user-1", "server-token").await;
    let store = FakeCredentialStore::available();
    let read = Arc::new(AtomicBool::new(false));
    let secrets = FakeSecretInput { read: read.clone() };
    let client_factory = WekanClientFactory::new(Some(server.uri()), false);

    let success = execute(args(), &client_factory, &store, &secrets)
        .await
        .unwrap();

    assert!(read.load(Ordering::SeqCst));
    let saved = store.saved.lock().unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].user_id, "user-1");
    assert_eq!(saved[0].token, "server-token");
    let CommandSuccess::Registration(output) = success else {
        panic!("expected registration success")
    };
    assert_eq!(output.user_id, "user-1");
    assert!(output.credential_stored);
    assert!(!format!("{output:?}").contains("server-token"));
}

#[tokio::test]
async fn credential_preflight_failure_happens_before_password_or_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::default();
    let read = Arc::new(AtomicBool::new(false));
    let secrets = FakeSecretInput { read: read.clone() };
    let client_factory = WekanClientFactory::new(Some(server.uri()), false);

    let error = execute(args(), &client_factory, &store, &secrets)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialStoreUnavailable);
    assert_eq!(error.exit_code(), StableExitCode::Credential);
    assert_eq!(error.details().account_created, Some(false));
    assert!(!read.load(Ordering::SeqCst));
}

#[tokio::test]
async fn existing_raw_credential_blocks_registration_before_password_or_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::available();
    store.saved.lock().unwrap().push(SavedCredential {
        account: "profile:test-store:default".to_owned(),
        user_id: "expired".to_owned(),
        token: "expired-or-malformed-entry".to_owned(),
    });
    let read = Arc::new(AtomicBool::new(false));
    let secrets = FakeSecretInput { read: read.clone() };
    let client_factory = WekanClientFactory::new(Some(server.uri()), false);

    let error = execute(args(), &client_factory, &store, &secrets)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialAlreadyExists);
    assert_eq!(error.exit_code(), StableExitCode::Configuration);
    assert_eq!(error.details().account_created, Some(false));
    assert_eq!(error.details().credential_stored, Some(true));
    assert!(!read.load(Ordering::SeqCst));
    server.verify().await;
}

#[tokio::test]
async fn rejected_registration_never_writes_credentials() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/register"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "username-already-exists",
            "reason": "Username already exists."
        })))
        .mount(&server)
        .await;
    let store = FakeCredentialStore::available();
    let read = Arc::new(AtomicBool::new(false));
    let secrets = FakeSecretInput { read };
    let client_factory = WekanClientFactory::new(Some(server.uri()), false);

    let error = execute(args(), &client_factory, &store, &secrets)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::RegistrationRejected);
    assert_eq!(error.exit_code(), StableExitCode::Server);
    assert!(store.saved.lock().unwrap().is_empty());
}

#[tokio::test]
async fn post_registration_store_failure_is_explicit_and_redacted() {
    let server = MockServer::start().await;
    mount_success(&server, "user-1", "server-token").await;
    let store = FakeCredentialStore::available();
    store.fail_save.store(true, Ordering::SeqCst);
    let read = Arc::new(AtomicBool::new(false));
    let secrets = FakeSecretInput { read };
    let client_factory = WekanClientFactory::new(Some(server.uri()), false);

    let error = execute(args(), &client_factory, &store, &secrets)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert_eq!(error.exit_code(), StableExitCode::Credential);
    assert_eq!(error.details().account_created, Some(true));
    assert!(!error.message().contains("server-token"));
    assert!(error.message().contains("[REDACTED]"));
}
