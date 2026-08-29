use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use clap::Parser;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

use super::{LoginArgs, execute as execute_target, map_client_error};
use crate::{
    cli::Cli,
    client::{ClientError, LoginRequest, WekanClientFactory},
    command_result::CommandSuccess,
    commands::{RootCommand, auth::AuthCommand},
    config::ResolvedTarget,
    credentials::{
        CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialMutation,
        CredentialRecord, CredentialStore, CredentialTarget, LoginSecretMode, LoginSecrets,
        SecretInputProvider,
    },
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    redaction::Redactor,
};

async fn execute(
    args: LoginArgs,
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
    mutation_locks: AtomicUsize,
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
        panic!("login must never delete credentials")
    }

    fn delete_if_matches(
        &self,
        _target: &CredentialTarget,
        _expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        panic!("login must never conditionally delete credentials")
    }

    fn lock_mutation(
        &self,
        target: &CredentialTarget,
    ) -> Result<Box<dyn CredentialMutation + Send + '_>, CredentialError> {
        self.mutation_locks.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(FakeCredentialMutation {
            store: self,
            target: target.clone(),
        }))
    }
}

struct FakeCredentialMutation<'a> {
    store: &'a FakeCredentialStore,
    target: CredentialTarget,
}

impl CredentialMutation for FakeCredentialMutation<'_> {
    fn exists(&self) -> Result<bool, CredentialError> {
        self.store.exists(&self.target)
    }

    fn load(&self) -> Result<Option<CredentialRecord>, CredentialError> {
        self.store.load(&self.target)
    }

    fn create(
        &self,
        record: &CredentialRecord,
    ) -> Result<CredentialCreateOutcome, CredentialError> {
        self.store.create(&self.target, record)
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        self.store.delete(&self.target)
    }

    fn delete_if_matches(
        &self,
        expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        self.store.delete_if_matches(&self.target, expected)
    }
}

struct FakeSecretInput {
    read: Arc<AtomicBool>,
    observed_mode: Mutex<Option<LoginSecretMode>>,
}

impl SecretInputProvider for FakeSecretInput {
    fn read_new_account_password(&self, _from_stdin: bool) -> Result<SecretString, AppError> {
        self.read.store(true, Ordering::SeqCst);
        Ok(SecretString::from("test-password".to_owned()))
    }

    fn read_login_secrets(&self, mode: LoginSecretMode) -> Result<LoginSecrets, AppError> {
        self.read.store(true, Ordering::SeqCst);
        *self.observed_mode.lock().unwrap() = Some(mode);
        let code = matches!(
            mode,
            LoginSecretMode::PromptPasswordAndCode | LoginSecretMode::StdinPasswordAndCode
        )
        .then(|| SecretString::from("123456".to_owned()));
        Ok(LoginSecrets::new(
            SecretString::from("test-password".to_owned()),
            code,
        ))
    }
}

fn secret_input(read: Arc<AtomicBool>) -> FakeSecretInput {
    FakeSecretInput {
        read,
        observed_mode: Mutex::new(None),
    }
}

fn args() -> LoginArgs {
    LoginArgs {
        username: Some("alice".to_owned()),
        email: None,
        password_stdin: false,
        code: false,
        code_stdin: false,
    }
}

async fn mount_success(server: &MockServer, token: &str) {
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "user-1",
            "token": token,
            "tokenExpires": "2030-01-02T03:04:05Z"
        })))
        .mount(server)
        .await;
}

#[test]
fn login_requires_exactly_one_nonempty_identity() {
    let missing = Cli::try_parse_from([
        "wekan",
        "--server",
        "https://wekan.example",
        "auth",
        "login",
        "--password-stdin",
    ])
    .expect_err("login without an identity must fail");
    assert!(missing.to_string().contains("--username <USERNAME>"));

    let both = Cli::try_parse_from([
        "wekan",
        "--server",
        "https://wekan.example",
        "auth",
        "login",
        "--username",
        "alice",
        "--email",
        "alice@example.com",
    ])
    .expect_err("login with two identities must fail");
    assert!(both.to_string().contains("cannot be used with"));

    let empty = Cli::try_parse_from([
        "wekan",
        "--server",
        "https://wekan.example",
        "auth",
        "login",
        "--username",
        "",
    ])
    .expect_err("an empty identity must fail");
    assert!(empty.to_string().contains("value must not be empty"));

    for identity in [
        ["--username", "alice"],
        ["--email", "not-validated-locally"],
    ] {
        Cli::try_parse_from([
            "wekan",
            "--server",
            "https://wekan.example",
            "auth",
            "login",
            identity[0],
            identity[1],
            "--password-stdin",
        ])
        .expect("either identity should be accepted");
    }
}

#[test]
fn two_factor_flags_enforce_secret_input_modes() {
    for invalid in [
        vec!["--username", "alice", "--code-stdin"],
        vec!["--username", "alice", "--password-stdin", "--code"],
        vec!["--username", "alice", "--code", "--code-stdin"],
    ] {
        let mut arguments = vec![
            "wekan",
            "--server",
            "https://wekan.example",
            "auth",
            "login",
        ];
        arguments.extend(invalid);
        Cli::try_parse_from(arguments).expect_err("invalid secret input mode must fail");
    }

    let cli = Cli::try_parse_from([
        "wekan",
        "--server",
        "https://wekan.example",
        "auth",
        "login",
        "--username",
        "alice",
        "--password-stdin",
        "--code-stdin",
    ])
    .expect("the two-line automation mode must parse");
    let RootCommand::Auth(auth) = cli.command else {
        panic!("expected auth command")
    };
    let AuthCommand::Login(args) = auth.command else {
        panic!("expected the login command")
    };
    assert_eq!(args.secret_mode(), LoginSecretMode::StdinPasswordAndCode);

    Cli::try_parse_from([
        "wekan",
        "--server",
        "https://wekan.example",
        "auth",
        "login",
        "--email",
        "alice@example.com",
        "--code",
    ])
    .expect("the interactive code prompt mode must parse");
}

#[test]
fn valid_flags_map_to_one_secret_input_mode() {
    for (password_stdin, code, code_stdin, expected) in [
        (false, false, false, LoginSecretMode::PromptPassword),
        (false, true, false, LoginSecretMode::PromptPasswordAndCode),
        (true, false, false, LoginSecretMode::StdinPassword),
        (true, false, true, LoginSecretMode::StdinPasswordAndCode),
    ] {
        let args = LoginArgs {
            username: Some("alice".to_owned()),
            email: None,
            password_stdin,
            code,
            code_stdin,
        };

        assert_eq!(args.secret_mode(), expected);
    }
}

#[test]
fn login_request_debug_does_not_expose_secrets() {
    let request = LoginRequest {
        username: Some("alice".to_owned()),
        email: None,
        password: SecretString::from("secret-password".to_owned()),
        code: Some(SecretString::from("654321".to_owned())),
    };
    let rendered = format!("{request:?}");

    assert!(!rendered.contains("secret-password"));
    assert!(!rendered.contains("654321"));
}

#[test]
fn login_errors_redact_password_and_two_factor_code() {
    let password = SecretString::from("secret-password".to_owned());
    let code = SecretString::from("654321".to_owned());
    let redactor = Redactor::with_secret(&password).and_secret(&code);
    let error = map_client_error(
        ClientError::Server {
            status: reqwest::StatusCode::UNAUTHORIZED,
            server_error: Some("bad secret-password".to_owned()),
            server_reason: Some("code 654321 rejected".to_owned()),
            retry_after_seconds: None,
        },
        &redactor,
    );

    assert_eq!(error.code(), ErrorCode::LoginRejected);
    let details = serde_json::to_string(error.details()).unwrap();
    assert!(!details.contains("secret-password"));
    assert!(!details.contains("654321"));
    assert_eq!(details.matches("[REDACTED]").count(), 2);
}

#[test]
fn login_statuses_map_to_the_stable_error_contract() {
    let password = SecretString::from("secret-password".to_owned());
    let redactor = Redactor::with_secret(&password);

    let malformed = map_client_error(
        ClientError::Server {
            status: reqwest::StatusCode::BAD_REQUEST,
            server_error: None,
            server_reason: None,
            retry_after_seconds: None,
        },
        &redactor,
    );
    assert_eq!(malformed.code(), ErrorCode::ProtocolError);
    assert_eq!(malformed.exit_code(), StableExitCode::Transport);
    assert_eq!(malformed.details().http_status, Some(400));

    let rejected = map_client_error(
        ClientError::Server {
            status: reqwest::StatusCode::UNAUTHORIZED,
            server_error: None,
            server_reason: None,
            retry_after_seconds: None,
        },
        &redactor,
    );
    assert_eq!(rejected.code(), ErrorCode::LoginRejected);
    assert_eq!(rejected.exit_code(), StableExitCode::Server);
    assert_eq!(rejected.details().http_status, Some(401));

    let limited = map_client_error(
        ClientError::Server {
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            server_error: Some("too-many-requests".to_owned()),
            server_reason: None,
            retry_after_seconds: Some(17),
        },
        &redactor,
    );
    assert_eq!(limited.code(), ErrorCode::LoginRateLimited);
    assert_eq!(limited.details().retry_after_seconds, Some(17));
    assert!(limited.message().contains("retry after 17 seconds"));

    let upstream = map_client_error(
        ClientError::Server {
            status: reqwest::StatusCode::BAD_GATEWAY,
            server_error: None,
            server_reason: None,
            retry_after_seconds: Some(29),
        },
        &redactor,
    );
    assert_eq!(upstream.code(), ErrorCode::ServerError);
    assert_eq!(upstream.details().outcome_unknown, Some(true));
    assert_eq!(upstream.details().retry_after_seconds, None);
}

#[test]
fn invalid_login_success_preserves_the_http_status() {
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
    assert_eq!(error.details().session_created, Some(true));
}

#[test]
fn missing_two_factor_code_is_actionable_and_machine_readable() {
    let password = SecretString::from("secret-password".to_owned());
    let redactor = Redactor::with_secret(&password);
    let error = map_client_error(
        ClientError::Server {
            status: reqwest::StatusCode::UNAUTHORIZED,
            server_error: Some("no-2fa-code".to_owned()),
            server_reason: Some("2FA code must be informed".to_owned()),
            retry_after_seconds: None,
        },
        &redactor,
    );

    assert_eq!(error.code(), ErrorCode::LoginRejected);
    assert_eq!(error.exit_code(), StableExitCode::Server);
    assert_eq!(error.details().two_factor_required, Some(true));
    assert_eq!(error.details().server_error.as_deref(), Some("no-2fa-code"));
    assert!(error.message().contains("--code"));
    assert!(error.message().contains("--code-stdin"));
}

#[test]
fn unreadable_login_responses_preserve_status_and_session_effects() {
    let password = SecretString::from("secret-password".to_owned());
    let redactor = Redactor::with_secret(&password);
    let success_error = map_client_error(
        ClientError::ResponseTooLarge {
            limit_bytes: 1024 * 1024,
            status: reqwest::StatusCode::OK,
            retry_after_seconds: None,
        },
        &redactor,
    );
    assert_eq!(success_error.code(), ErrorCode::ProtocolError);
    assert_eq!(success_error.details().session_created, Some(true));

    let malformed = map_client_error(
        ClientError::ResponseTooLarge {
            limit_bytes: 1024 * 1024,
            status: reqwest::StatusCode::BAD_REQUEST,
            retry_after_seconds: None,
        },
        &redactor,
    );
    assert_eq!(malformed.code(), ErrorCode::ProtocolError);
    assert_eq!(malformed.exit_code(), StableExitCode::Transport);
    assert_eq!(malformed.details().http_status, Some(400));

    let limited = map_client_error(
        ClientError::ResponseTooLarge {
            limit_bytes: 1024 * 1024,
            status: reqwest::StatusCode::TOO_MANY_REQUESTS,
            retry_after_seconds: Some(23),
        },
        &redactor,
    );
    assert_eq!(limited.code(), ErrorCode::LoginRateLimited);
    assert_eq!(limited.details().retry_after_seconds, Some(23));

    let upstream = map_client_error(
        ClientError::ResponseTooLarge {
            limit_bytes: 1024 * 1024,
            status: reqwest::StatusCode::GATEWAY_TIMEOUT,
            retry_after_seconds: None,
        },
        &redactor,
    );
    assert_eq!(upstream.code(), ErrorCode::ServerError);
    assert_eq!(upstream.details().outcome_unknown, Some(true));
}

#[tokio::test]
async fn successful_login_creates_a_credential_and_returns_only_metadata() {
    let server = MockServer::start().await;
    mount_success(&server, "new-token").await;
    let store = FakeCredentialStore::available();
    let read = Arc::new(AtomicBool::new(false));
    let secrets = secret_input(read.clone());
    let client_factory = WekanClientFactory::new(Some(server.uri()), false);

    let success = execute(args(), &client_factory, &store, &secrets)
        .await
        .unwrap();

    assert!(read.load(Ordering::SeqCst));
    assert_eq!(
        *secrets.observed_mode.lock().unwrap(),
        Some(LoginSecretMode::PromptPassword)
    );
    assert_eq!(store.mutation_locks.load(Ordering::SeqCst), 1);
    let saved = store.saved.lock().unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].user_id, "user-1");
    assert_eq!(saved[0].token, "new-token");
    let CommandSuccess::Login(output) = success else {
        panic!("expected login success")
    };
    assert_eq!(output.user_id, "user-1");
    assert!(output.credential_stored);
    assert!(!format!("{output:?}").contains("new-token"));
}

#[tokio::test]
async fn same_server_profiles_store_independent_credentials_and_ignore_legacy_url_entries() {
    let server = MockServer::start().await;
    mount_success(&server, "profile-token").await;
    let store = FakeCredentialStore::available();
    let direct_account = format!("{}/", server.uri());
    store.saved.lock().unwrap().push(SavedCredential {
        account: direct_account.clone(),
        user_id: "direct-user".to_owned(),
        token: "direct-token".to_owned(),
    });

    for profile in ["work", "personal"] {
        let secrets = secret_input(Arc::new(AtomicBool::new(false)));
        let factory = WekanClientFactory::for_profile(server.uri(), profile.to_owned(), false);
        let success = execute(args(), &factory, &store, &secrets).await.unwrap();
        let CommandSuccess::Login(output) = success else {
            panic!("expected login success")
        };
        assert_eq!(output.profile, profile);
    }

    let saved = store.saved.lock().unwrap();
    assert_eq!(saved.len(), 3);
    assert!(saved.iter().any(|record| record.account == direct_account));
    assert!(
        saved
            .iter()
            .any(|record| record.account == "profile:test-store:work")
    );
    assert!(
        saved
            .iter()
            .any(|record| record.account == "profile:test-store:personal")
    );
}

#[tokio::test]
async fn credential_preflight_failure_happens_before_secrets_or_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::default();
    let read = Arc::new(AtomicBool::new(false));
    let secrets = secret_input(read.clone());
    let client_factory = WekanClientFactory::new(Some(server.uri()), false);

    let error = execute(args(), &client_factory, &store, &secrets)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialStoreUnavailable);
    assert_eq!(error.details().session_created, Some(false));
    assert!(!read.load(Ordering::SeqCst));
}

#[tokio::test]
async fn existing_raw_credential_blocks_login_before_secrets_or_http() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::available();
    store.saved.lock().unwrap().push(SavedCredential {
        account: "profile:test-store:default".to_owned(),
        user_id: "unreadable".to_owned(),
        token: "malformed-raw-entry".to_owned(),
    });
    let read = Arc::new(AtomicBool::new(false));
    let secrets = secret_input(read.clone());
    let client_factory = WekanClientFactory::new(Some(server.uri()), false);

    let error = execute(args(), &client_factory, &store, &secrets)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialAlreadyExists);
    assert_eq!(error.exit_code(), StableExitCode::Configuration);
    assert_eq!(error.details().session_created, Some(false));
    assert_eq!(error.details().credential_stored, Some(true));
    assert!(!read.load(Ordering::SeqCst));
    server.verify().await;
}

#[tokio::test]
async fn rejected_login_never_writes_credentials() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "login-failed",
            "reason": "Incorrect username, email address or password."
        })))
        .mount(&server)
        .await;
    let store = FakeCredentialStore::available();
    let secrets = secret_input(Arc::new(AtomicBool::new(false)));
    let client_factory = WekanClientFactory::new(Some(server.uri()), false);

    let error = execute(args(), &client_factory, &store, &secrets)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::LoginRejected);
    assert!(store.saved.lock().unwrap().is_empty());
}

#[tokio::test]
async fn missing_two_factor_code_is_reported_without_replaying_login() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/users/login"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": "no-2fa-code",
            "reason": "2FA code must be informed"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let store = FakeCredentialStore::available();
    let secrets = secret_input(Arc::new(AtomicBool::new(false)));
    let client_factory = WekanClientFactory::new(Some(server.uri()), false);

    let error = execute(args(), &client_factory, &store, &secrets)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::LoginRejected);
    assert_eq!(error.details().two_factor_required, Some(true));
    assert!(error.message().contains("--code"));
    assert!(store.saved.lock().unwrap().is_empty());
    server.verify().await;
}

#[tokio::test]
async fn post_login_store_failure_reports_a_created_session_and_redacts_the_token() {
    let server = MockServer::start().await;
    mount_success(&server, "new-token").await;
    let store = FakeCredentialStore::available();
    store.fail_save.store(true, Ordering::SeqCst);
    let secrets = secret_input(Arc::new(AtomicBool::new(false)));
    let client_factory = WekanClientFactory::new(Some(server.uri()), false);

    let error = execute(args(), &client_factory, &store, &secrets)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
    assert_eq!(error.exit_code(), StableExitCode::Credential);
    assert_eq!(error.details().session_created, Some(true));
    assert!(!error.message().contains("new-token"));
    assert!(error.message().contains("[REDACTED]"));
}
