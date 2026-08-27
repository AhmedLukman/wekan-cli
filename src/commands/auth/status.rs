use clap::Args;
use reqwest::StatusCode;

use crate::commands::{
    authenticated::AuthenticatedContext,
    client_error::{
        embedded_protocol_error_details, embedded_server_error_details, protocol_error_details,
        response_error_details, server_error_details,
    },
};
use crate::{
    client::{ClientError, WekanClientFactory},
    command_result::{AuthStatusEmail, AuthStatusSuccess, AuthStatusUser, CommandSuccess},
    credentials::CredentialStore,
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    redaction::Redactor,
};

const REJECTED_CREDENTIAL_MESSAGE: &str = "the stored credential was rejected by Wekan; remove it with `wekan auth logout --local-only`, then log in again to create a new session";

#[derive(Debug, Args)]
pub struct StatusArgs {}

pub(crate) async fn execute(
    _args: StatusArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let current_user = context
        .client()
        .current_user(context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor))?;

    if current_user.user_id() != context.record().user_id() {
        return Err(AppError::new(
            ErrorCode::CredentialStoreFailed,
            "the stored credential user id did not match the authenticated Wekan user",
            StableExitCode::Credential,
        ));
    }

    let (user_id, username, full_name, is_admin, emails) = current_user.into_auth_parts();
    Ok(CommandSuccess::AuthStatus(AuthStatusSuccess {
        server: context.server().to_owned(),
        profile: context.profile().to_owned(),
        authenticated: true,
        token_expires: context.token_expires().to_owned(),
        credential_stored: true,
        user: AuthStatusUser {
            user_id,
            username,
            full_name,
            is_admin,
            emails: emails
                .into_iter()
                .map(|email| {
                    let (address, verified) = email.into_parts();
                    AuthStatusEmail { address, verified }
                })
                .collect(),
        },
    }))
}

fn map_client_error(error: ClientError, redactor: &Redactor) -> AppError {
    match error {
        ClientError::Build(_) => AppError::new(
            ErrorCode::InternalError,
            "the HTTP client could not be initialized",
            StableExitCode::Internal,
        ),
        ClientError::Transport(error) => AppError::new(
            ErrorCode::TransportError,
            redactor.redact(&format!(
                "the authentication status request could not be completed: {error}"
            )),
            StableExitCode::Transport,
        ),
        ClientError::UnexpectedRedirect { status } => AppError::new(
            ErrorCode::UnexpectedRedirect,
            "the Wekan server returned a redirect; redirects are not followed for authenticated requests",
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
            "the server response body could not be read".to_owned(),
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
        .with_details(protocol_error_details(success_status_received)),
        ClientError::EmbeddedProtocol {
            http_status,
            server_error,
            server_reason,
            server_message,
            server_error_type,
            server_is_client_safe,
        } => AppError::new(
            ErrorCode::ProtocolError,
            "Wekan returned an embedded authentication-status error without statusCode",
            StableExitCode::Server,
        )
        .with_details(embedded_protocol_error_details(
            http_status,
            server_error,
            server_reason,
            server_message,
            server_error_type,
            server_is_client_safe,
            redactor,
        )),
        ClientError::Server {
            status,
            server_error,
            server_reason,
            retry_after_seconds,
        } => {
            let authentication_rejected = status == StatusCode::UNAUTHORIZED;
            AppError::new(
                if authentication_rejected {
                    ErrorCode::AuthenticationRejected
                } else {
                    ErrorCode::ServerError
                },
                if authentication_rejected {
                    REJECTED_CREDENTIAL_MESSAGE
                } else {
                    "the Wekan server returned an unexpected error while checking authentication status"
                },
                StableExitCode::Server,
            )
            .with_details(server_error_details(
                status,
                server_error,
                server_reason,
                retry_after_seconds,
                redactor,
            ))
        }
        ClientError::EmbeddedServer {
            http_status,
            wekan_status_code,
            server_error,
            server_reason,
        } => {
            let authentication_rejected = wekan_status_code == StatusCode::UNAUTHORIZED.as_u16();
            AppError::new(
                if authentication_rejected {
                    ErrorCode::AuthenticationRejected
                } else {
                    ErrorCode::ServerError
                },
                if authentication_rejected {
                    REJECTED_CREDENTIAL_MESSAGE
                } else {
                    "the Wekan server returned an embedded error while checking authentication status"
                },
                StableExitCode::Server,
            )
            .with_details(embedded_server_error_details(
                http_status,
                wekan_status_code,
                server_error,
                server_reason,
                redactor,
            ))
        }
    }
}

fn response_body_error(
    message: String,
    status: StatusCode,
    retry_after_seconds: Option<u64>,
) -> AppError {
    let details = response_error_details(status, retry_after_seconds);
    if status == StatusCode::OK {
        return AppError::new(ErrorCode::ProtocolError, message, StableExitCode::Transport)
            .with_details(details);
    }

    let authentication_rejected = status == StatusCode::UNAUTHORIZED;
    AppError::new(
        if authentication_rejected {
            ErrorCode::AuthenticationRejected
        } else {
            ErrorCode::ServerError
        },
        if authentication_rejected {
            REJECTED_CREDENTIAL_MESSAGE.to_owned()
        } else {
            message
        },
        StableExitCode::Server,
    )
    .with_details(details)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use clap::Parser;
    use secrecy::SecretString;
    use serde_json::json;
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
    };

    use super::{StatusArgs, map_client_error, response_body_error};
    use crate::{
        cli::Cli,
        client::{ClientError, WekanClientFactory},
        command_result::CommandSuccess,
        commands::{RootCommand, auth::AuthCommand},
        credentials::{
            CredentialCreateOutcome, CredentialError, CredentialRecord, CredentialStore,
            CredentialTarget,
        },
        error::{AppError, ErrorCode},
        exit_code::StableExitCode,
        redaction::Redactor,
    };

    async fn execute(
        args: StatusArgs,
        client_factory: &WekanClientFactory,
        credential_store: &dyn CredentialStore,
    ) -> Result<CommandSuccess, AppError> {
        super::execute(args, client_factory, credential_store).await
    }

    struct FakeCredentialStore {
        available: bool,
        record: Mutex<Option<CredentialRecord>>,
        load_error: Mutex<Option<CredentialError>>,
    }

    impl FakeCredentialStore {
        fn with_record(record: CredentialRecord) -> Self {
            Self {
                available: true,
                record: Mutex::new(Some(record)),
                load_error: Mutex::new(None),
            }
        }

        fn missing() -> Self {
            Self {
                available: true,
                record: Mutex::new(None),
                load_error: Mutex::new(None),
            }
        }
    }

    impl CredentialStore for FakeCredentialStore {
        fn check_available(&self, _target: &CredentialTarget) -> Result<(), CredentialError> {
            if self.available {
                Ok(())
            } else {
                Err(CredentialError::Unavailable("test unavailable".to_owned()))
            }
        }

        fn exists(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
            Ok(self.record.lock().unwrap().is_some())
        }

        fn load(
            &self,
            _target: &CredentialTarget,
        ) -> Result<Option<CredentialRecord>, CredentialError> {
            if let Some(error) = self.load_error.lock().unwrap().take() {
                return Err(error);
            }
            Ok(self.record.lock().unwrap().clone())
        }

        fn create(
            &self,
            _target: &CredentialTarget,
            _record: &CredentialRecord,
        ) -> Result<CredentialCreateOutcome, CredentialError> {
            panic!("authentication status must never save credentials")
        }

        fn delete(&self, _target: &CredentialTarget) -> Result<bool, CredentialError> {
            panic!("authentication status must never delete credentials")
        }

        fn delete_if_matches(
            &self,
            _target: &CredentialTarget,
            _expected: &CredentialRecord,
        ) -> Result<crate::credentials::CredentialDeleteOutcome, CredentialError> {
            panic!("authentication status must never conditionally delete credentials")
        }
    }

    fn record(server: &MockServer, user_id: &str, token: &str, expiry: &str) -> CredentialRecord {
        CredentialRecord::new(
            format!("{}/", server.uri()),
            user_id.to_owned(),
            SecretString::from(token.to_owned()),
            OffsetDateTime::parse(expiry, &Rfc3339).unwrap(),
        )
    }

    #[test]
    fn status_is_argument_free() {
        let cli = Cli::try_parse_from([
            "wekan",
            "--server",
            "https://wekan.example",
            "auth",
            "status",
        ])
        .expect("status without arguments must parse");
        let RootCommand::Auth(auth) = cli.command else {
            panic!("expected auth command")
        };
        assert!(matches!(auth.command, AuthCommand::Status(StatusArgs {})));

        let error = Cli::try_parse_from([
            "wekan",
            "--server",
            "https://wekan.example",
            "auth",
            "status",
            "unexpected",
        ])
        .expect_err("status-specific arguments must be rejected");
        assert!(error.to_string().contains("unexpected"));
    }

    #[test]
    fn status_only_exposes_retry_metadata_for_rate_limits() {
        let token = SecretString::from("status-token".to_owned());
        let redactor = Redactor::with_secret(&token);
        let unavailable = map_client_error(
            ClientError::Server {
                status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
                server_error: None,
                server_reason: None,
                retry_after_seconds: Some(41),
            },
            &redactor,
        );
        let oversized = map_client_error(
            ClientError::ResponseTooLarge {
                limit_bytes: 1024 * 1024,
                status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
                retry_after_seconds: Some(43),
            },
            &redactor,
        );
        let limited = map_client_error(
            ClientError::Server {
                status: reqwest::StatusCode::TOO_MANY_REQUESTS,
                server_error: None,
                server_reason: None,
                retry_after_seconds: Some(47),
            },
            &redactor,
        );

        assert_eq!(unavailable.details().retry_after_seconds, None);
        assert_eq!(oversized.details().retry_after_seconds, None);
        assert_eq!(limited.details().retry_after_seconds, Some(47));
    }

    #[test]
    fn rejected_credentials_require_removal_before_login() {
        let token = SecretString::from("status-token".to_owned());
        let redactor = Redactor::with_secret(&token);
        let errors = [
            map_client_error(
                ClientError::Server {
                    status: reqwest::StatusCode::UNAUTHORIZED,
                    server_error: None,
                    server_reason: None,
                    retry_after_seconds: None,
                },
                &redactor,
            ),
            map_client_error(
                ClientError::EmbeddedServer {
                    http_status: reqwest::StatusCode::OK,
                    wekan_status_code: reqwest::StatusCode::UNAUTHORIZED.as_u16(),
                    server_error: None,
                    server_reason: None,
                },
                &redactor,
            ),
            response_body_error(
                "unreadable rejection".to_owned(),
                reqwest::StatusCode::UNAUTHORIZED,
                None,
            ),
        ];

        for error in errors {
            assert_eq!(error.code(), ErrorCode::AuthenticationRejected);
            assert!(error.message().contains("auth logout --local-only"));
            assert!(error.message().contains("then log in again"));
        }
    }

    #[tokio::test]
    async fn missing_credentials_return_a_stable_error_without_http() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/user"))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&server)
            .await;
        let factory = WekanClientFactory::new(Some(server.uri()), false);

        let error = execute(StatusArgs {}, &factory, &FakeCredentialStore::missing())
            .await
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::CredentialNotFound);
        assert_eq!(error.exit_code(), StableExitCode::Server);
        assert_eq!(error.details(), &Default::default());
    }

    #[tokio::test]
    async fn expired_credentials_return_expiry_without_http() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/user"))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&server)
            .await;
        let factory = WekanClientFactory::new(Some(server.uri()), false);
        let store = FakeCredentialStore::with_record(record(
            &server,
            "user-1",
            "expired-token",
            "2000-01-02T03:04:05Z",
        ));

        let error = execute(StatusArgs {}, &factory, &store).await.unwrap_err();

        assert_eq!(error.code(), ErrorCode::CredentialExpired);
        assert_eq!(error.exit_code(), StableExitCode::Server);
        assert_eq!(
            error.details().token_expires.as_deref(),
            Some("2000-01-02T03:04:05Z")
        );
        assert!(error.message().contains("auth logout --local-only"));
        assert!(error.message().contains("then log in again"));
    }

    #[tokio::test]
    async fn valid_credentials_return_only_the_allowlisted_profile() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/user"))
            .and(header("authorization", "Bearer status-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_id": "user-1",
                "username": "alice",
                "emails": [{"address": "alice@example.com", "verified": true}],
                "profile": {"fullname": "Alice Example", "privateValue": "ignore-me"},
                "isAdmin": false,
                "boards": [{"boardId": "secret-board"}],
                "services": {"resume": {"loginTokens": ["never-output"]}}
            })))
            .expect(1)
            .mount(&server)
            .await;
        let factory = WekanClientFactory::new(Some(server.uri()), false);
        let store = FakeCredentialStore::with_record(record(
            &server,
            "user-1",
            "status-token",
            "2999-01-02T03:04:05Z",
        ));

        let success = execute(StatusArgs {}, &factory, &store).await.unwrap();
        let CommandSuccess::AuthStatus(status) = success else {
            panic!("expected authentication status success")
        };

        assert!(status.authenticated);
        assert!(status.credential_stored);
        assert_eq!(status.user.user_id, "user-1");
        assert_eq!(status.user.username.as_deref(), Some("alice"));
        assert_eq!(status.user.full_name.as_deref(), Some("Alice Example"));
        assert_eq!(status.user.is_admin, Some(false));
        assert_eq!(status.user.emails[0].verified, Some(true));
        let rendered = serde_json::to_string(&status).unwrap();
        assert!(!rendered.contains("status-token"));
        assert!(!rendered.contains("secret-board"));
        assert!(!rendered.contains("never-output"));
        assert!(!rendered.contains("privateValue"));
    }

    #[tokio::test]
    async fn embedded_authentication_rejections_are_accurate_and_redacted() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/user"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "error": "Unauthorized status-token",
                "reason": "status-token was rejected",
                "statusCode": 401
            })))
            .mount(&server)
            .await;
        let factory = WekanClientFactory::new(Some(server.uri()), false);
        let store = FakeCredentialStore::with_record(record(
            &server,
            "user-1",
            "status-token",
            "2999-01-02T03:04:05Z",
        ));

        let error = execute(StatusArgs {}, &factory, &store).await.unwrap_err();

        assert_eq!(error.code(), ErrorCode::AuthenticationRejected);
        assert_eq!(error.exit_code(), StableExitCode::Server);
        assert_eq!(error.details().http_status, Some(200));
        assert_eq!(error.details().wekan_status_code, Some(401));
        let rendered = serde_json::to_string(error.details()).unwrap();
        assert!(!rendered.contains("status-token"));
        assert!(rendered.contains("[REDACTED]"));
    }

    #[tokio::test]
    async fn a_remote_user_mismatch_is_a_credential_failure() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/user"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "_id": "different-user"
            })))
            .mount(&server)
            .await;
        let factory = WekanClientFactory::new(Some(server.uri()), false);
        let store = FakeCredentialStore::with_record(record(
            &server,
            "user-1",
            "status-token",
            "2999-01-02T03:04:05Z",
        ));

        let error = execute(StatusArgs {}, &factory, &store).await.unwrap_err();

        assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
        assert_eq!(error.exit_code(), StableExitCode::Credential);
    }

    #[tokio::test]
    async fn credential_load_failures_do_not_contact_wekan() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/user"))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&server)
            .await;
        let factory = WekanClientFactory::new(Some(server.uri()), false);
        let store = FakeCredentialStore {
            available: true,
            record: Mutex::new(None),
            load_error: Mutex::new(Some(CredentialError::InvalidRecord("test invalid"))),
        };

        let error = execute(StatusArgs {}, &factory, &store).await.unwrap_err();

        assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
        assert_eq!(error.exit_code(), StableExitCode::Credential);
    }
}
