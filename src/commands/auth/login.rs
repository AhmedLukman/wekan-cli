use clap::Args;

use crate::{
    client::{ClientError, LoginRequest, WekanClientFactory},
    credentials::{CredentialStore, LoginSecretMode, SecretInputProvider},
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    output::CommandSuccess,
    redaction::Redactor,
};

use super::{
    embedded_server_error_details, non_empty_identity, persist_session, preflight_credentials,
    server_error_details,
};

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// Username for the account.
    #[arg(
        long,
        value_parser = non_empty_identity,
        required_unless_present = "email",
        conflicts_with = "email"
    )]
    pub username: Option<String>,

    /// Email address for the account.
    #[arg(
        long,
        value_parser = non_empty_identity,
        required_unless_present = "username",
        conflicts_with = "username"
    )]
    pub email: Option<String>,

    /// Read the password from the first line of standard input.
    #[arg(long)]
    pub password_stdin: bool,

    /// Prompt without echo for a two-factor authentication code.
    #[arg(long, conflicts_with_all = ["password_stdin", "code_stdin"])]
    pub code: bool,

    /// Read the two-factor code from the line after the password on standard input.
    #[arg(long, requires = "password_stdin", conflicts_with = "code")]
    pub code_stdin: bool,
}

impl LoginArgs {
    const fn secret_mode(&self) -> LoginSecretMode {
        if self.password_stdin {
            if self.code_stdin {
                LoginSecretMode::StdinPasswordAndCode
            } else {
                LoginSecretMode::StdinPassword
            }
        } else if self.code {
            LoginSecretMode::PromptPasswordAndCode
        } else {
            LoginSecretMode::PromptPassword
        }
    }
}

pub(crate) async fn execute(
    args: LoginArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    let client = client_factory.create().map_err(AppError::from)?;
    let server_url = client.server().as_str().to_owned();

    preflight_credentials(credential_store, &server_url).map_err(|error| {
        error.with_details(ErrorDetails {
            session_created: Some(false),
            ..ErrorDetails::default()
        })
    })?;

    let secrets = secret_input.read_login_secrets(args.secret_mode())?;
    let (password, code) = secrets.into_parts();
    let request = LoginRequest {
        username: args.username,
        email: args.email,
        password,
        code,
    };
    let mut redactor = Redactor::with_secret(&request.password);
    if let Some(code) = request.code.as_ref() {
        redactor = redactor.and_secret(code);
    }
    let session = client
        .login(&request)
        .await
        .map_err(|error| map_client_error(error, &redactor))?;

    let success = persist_session(credential_store, server_url, session).map_err(|error| {
        error.with_details(ErrorDetails {
            session_created: Some(true),
            ..ErrorDetails::default()
        })
    })?;
    Ok(CommandSuccess::Login(success))
}

fn map_client_error(error: ClientError, redactor: &Redactor<'_>) -> AppError {
    match error {
        ClientError::Build(_) => AppError::new(
            ErrorCode::InternalError,
            "the HTTP client could not be initialized",
            StableExitCode::Internal,
        ),
        ClientError::Transport(_) => AppError::new(
            ErrorCode::TransportError,
            "the login request failed; its remote outcome may be unknown, so check the server before retrying",
            StableExitCode::Transport,
        )
        .with_details(ErrorDetails {
            outcome_unknown: Some(true),
            ..ErrorDetails::default()
        }),
        ClientError::UnexpectedRedirect { status } => AppError::new(
            ErrorCode::UnexpectedRedirect,
            "the server redirected login; configure the canonical Wekan URL because login requests are never replayed",
            StableExitCode::Transport,
        )
        .with_details(ErrorDetails {
            http_status: Some(status.as_u16()),
            ..ErrorDetails::default()
        }),
        ClientError::ResponseTooLarge {
            limit_bytes,
            status,
            retry_after_seconds,
        } => response_body_error(
            format!("the server response exceeded the {limit_bytes}-byte safety limit"),
            status,
            retry_after_seconds,
        ),
        ClientError::ResponseBody {
            status,
            retry_after_seconds,
            ..
        } => response_body_error(
            "the server response body could not be read",
            status,
            retry_after_seconds,
        ),
        ClientError::Protocol {
            message,
            success_status_received,
        } => AppError::new(
            ErrorCode::ProtocolError,
            redactor.redact(&format!("invalid response from Wekan: {message}")),
            StableExitCode::Transport,
        )
        .with_details(ErrorDetails {
            session_created: Some(success_status_received),
            ..ErrorDetails::default()
        }),
        ClientError::Server {
            status,
            server_error,
            server_reason,
            retry_after_seconds,
        } => {
            let two_factor_required = status == reqwest::StatusCode::UNAUTHORIZED
                && server_error.as_deref() == Some("no-2fa-code");
            let (code, message, exit_code, outcome_unknown) = match status.as_u16() {
                400 => (
                    ErrorCode::ProtocolError,
                    "Wekan rejected the login request as malformed",
                    StableExitCode::Transport,
                    None,
                ),
                401 if two_factor_required => (
                    ErrorCode::LoginRejected,
                    "this account requires a two-factor code; retry with --code or --code-stdin",
                    StableExitCode::Server,
                    None,
                ),
                401 => (
                    ErrorCode::LoginRejected,
                    "Wekan rejected the username or email, password, or two-factor code",
                    StableExitCode::Server,
                    None,
                ),
                429 => (
                    ErrorCode::LoginRateLimited,
                    "too many failed login attempts; try again later",
                    StableExitCode::Server,
                    None,
                ),
                _ if status.is_server_error() => (
                    ErrorCode::ServerError,
                    "the Wekan server returned an unexpected error; the remote outcome may be unknown, so check the server before retrying",
                    StableExitCode::Server,
                    Some(true),
                ),
                _ => (
                    ErrorCode::ServerError,
                    "the Wekan server returned an unexpected error",
                    StableExitCode::Server,
                    None,
                ),
            };
            let message = append_retry_after(message.to_owned(), status, retry_after_seconds);
            let mut details = server_error_details(
                status,
                server_error,
                server_reason,
                (status == reqwest::StatusCode::TOO_MANY_REQUESTS)
                    .then_some(retry_after_seconds)
                    .flatten(),
                redactor,
            );
            details.two_factor_required = two_factor_required.then_some(true);
            details.outcome_unknown = outcome_unknown;
            AppError::new(code, message, exit_code).with_details(details)
        }
        ClientError::EmbeddedServer {
            http_status,
            wekan_status_code,
            server_error,
            server_reason,
        } => AppError::new(
            ErrorCode::ServerError,
            "the Wekan server returned an unexpected embedded login error",
            StableExitCode::Server,
        )
        .with_details(embedded_server_error_details(
            http_status,
            wekan_status_code,
            server_error,
            server_reason,
            redactor,
        )),
    }
}

fn response_body_error(
    message: impl Into<String>,
    status: reqwest::StatusCode,
    retry_after_seconds: Option<u64>,
) -> AppError {
    let message = message.into();
    if status == reqwest::StatusCode::OK {
        return AppError::new(ErrorCode::ProtocolError, message, StableExitCode::Transport)
            .with_details(ErrorDetails {
                http_status: Some(status.as_u16()),
                session_created: Some(true),
                ..ErrorDetails::default()
            });
    }

    let (code, exit_code, outcome_unknown) = match status.as_u16() {
        400 => (ErrorCode::ProtocolError, StableExitCode::Transport, None),
        401 => (ErrorCode::LoginRejected, StableExitCode::Server, None),
        429 => (ErrorCode::LoginRateLimited, StableExitCode::Server, None),
        _ if status.is_server_error() => {
            (ErrorCode::ServerError, StableExitCode::Server, Some(true))
        }
        _ => (ErrorCode::ServerError, StableExitCode::Server, None),
    };
    let mut message = if status == reqwest::StatusCode::UNAUTHORIZED {
        "Wekan rejected the username or email, password, or two-factor code".to_owned()
    } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        "too many failed login attempts; try again later".to_owned()
    } else {
        message
    };
    if outcome_unknown.is_some() {
        message
            .push_str("; the remote outcome may be unknown, so check the server before retrying");
    }
    message = append_retry_after(message, status, retry_after_seconds);

    AppError::new(code, message, exit_code).with_details(ErrorDetails {
        http_status: Some(status.as_u16()),
        outcome_unknown,
        retry_after_seconds: (status == reqwest::StatusCode::TOO_MANY_REQUESTS)
            .then_some(retry_after_seconds)
            .flatten(),
        ..ErrorDetails::default()
    })
}

fn append_retry_after(
    mut message: String,
    status: reqwest::StatusCode,
    retry_after_seconds: Option<u64>,
) -> String {
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS
        && let Some(seconds) = retry_after_seconds
    {
        message.push_str(&format!("; retry after {seconds} seconds"));
    }
    message
}

#[cfg(test)]
mod tests {
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

    use super::{LoginArgs, execute, map_client_error};
    use crate::{
        cli::Cli,
        client::{ClientError, LoginRequest, WekanClientFactory},
        commands::{RootCommand, auth::AuthCommand},
        credentials::{
            CredentialError, CredentialRecord, CredentialStore, LoginSecretMode, LoginSecrets,
            SecretInputProvider,
        },
        error::{AppError, ErrorCode},
        exit_code::StableExitCode,
        output::CommandSuccess,
        redaction::Redactor,
    };

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
        fn check_available(&self, _account: &str) -> Result<(), CredentialError> {
            if self.available.load(Ordering::SeqCst) {
                Ok(())
            } else {
                Err(CredentialError::Unavailable("test unavailable".to_owned()))
            }
        }

        fn load(&self, _account: &str) -> Result<Option<CredentialRecord>, CredentialError> {
            Ok(None)
        }

        fn save(&self, account: &str, record: &CredentialRecord) -> Result<(), CredentialError> {
            if self.fail_save.load(Ordering::SeqCst) {
                return Err(CredentialError::Store(format!(
                    "failed while handling {}",
                    record.token().expose_secret()
                )));
            }
            let mut saved = self.saved.lock().unwrap();
            saved.retain(|credential| credential.account != account);
            saved.push(SavedCredential {
                account: account.to_owned(),
                user_id: record.user_id().to_owned(),
                token: record.token().expose_secret().to_owned(),
            });
            Ok(())
        }
    }

    struct FakeSecretInput {
        read: Arc<AtomicBool>,
        observed_mode: Mutex<Option<LoginSecretMode>>,
    }

    impl SecretInputProvider for FakeSecretInput {
        fn read_registration_password(&self, _from_stdin: bool) -> Result<SecretString, AppError> {
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
        let RootCommand::Auth(auth) = cli.command;
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
                retry_after_seconds: None,
            },
            &redactor,
        );
        assert_eq!(upstream.code(), ErrorCode::ServerError);
        assert_eq!(upstream.details().outcome_unknown, Some(true));
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
    async fn successful_login_replaces_the_stored_credential_and_returns_only_metadata() {
        let server = MockServer::start().await;
        mount_success(&server, "new-token").await;
        let store = FakeCredentialStore::available();
        store.saved.lock().unwrap().push(SavedCredential {
            account: format!("{}/", server.uri()),
            user_id: "old-user".to_owned(),
            token: "old-token".to_owned(),
        });
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
}
