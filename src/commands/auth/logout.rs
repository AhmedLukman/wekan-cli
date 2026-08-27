use clap::Args;
use reqwest::StatusCode;

use crate::commands::{
    client_error::{
        embedded_protocol_error_details, embedded_server_error_details, protocol_error_details,
        response_error_details, server_error_details,
    },
    credential_ops::{
        credential_target, lock_credential_mutation, map_credential_load_error,
        preflight_credentials,
    },
};
use crate::{
    client::{ClientError, LogoutRequest, WekanClientFactory},
    command_result::{
        CancellationSuccess, CommandSuccess, DestructiveOperation, LogoutScope, LogoutSuccess,
    },
    credentials::{CredentialDeleteOutcome, CredentialMutation, CredentialRecord, CredentialStore},
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip,
    },
    redaction::Redactor,
};

#[derive(Debug, Args)]
pub struct LogoutArgs {
    /// Revoke every login token for the authenticated Wekan user.
    #[arg(long, conflicts_with = "local_only")]
    pub all: bool,

    /// Remove only the local credential without contacting Wekan. With --server,
    /// this can also clear an orphaned credential for a missing profile.
    #[arg(long, conflicts_with = "all")]
    pub local_only: bool,

    #[command(flatten)]
    pub confirmation: ConfirmationArgs,
}

pub(crate) async fn execute(
    args: LogoutArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    if args.local_only {
        let server = client_factory.server_identity().as_str().to_owned();
        let credential_target = credential_target(
            client_factory
                .profile()
                .expect("authentication requires a named profile"),
            client_factory
                .profile_store_namespace()
                .expect("authentication requires a profile credential namespace"),
            server.clone(),
        );
        let credential_mutation = lock_credential_mutation(credential_store, &credential_target)
            .map_err(|error| {
                error.with_details(no_remote_mutation_details(LogoutScope::LocalOnly, None))
            })?;
        if let Some(cancelled) = confirm_logout(
            &args,
            client_factory,
            &server,
            LogoutScope::LocalOnly,
            confirmation,
        )? {
            return Ok(cancelled);
        }
        preflight_credentials(credential_store, &credential_target).map_err(|error| {
            error.with_details(no_remote_mutation_details(LogoutScope::LocalOnly, None))
        })?;
        return execute_local_only(client_factory, credential_mutation.as_ref());
    }

    let scope = if args.all {
        LogoutScope::AllTokens
    } else {
        LogoutScope::CurrentToken
    };
    let client = client_factory.create().map_err(|error| {
        AppError::from(error).with_details(no_remote_mutation_details(scope, None))
    })?;
    let server = client.server().as_str().to_owned();
    let credential_target = credential_target(
        client_factory
            .profile()
            .expect("authentication requires a named profile"),
        client_factory
            .profile_store_namespace()
            .expect("authentication requires a profile credential namespace"),
        server.clone(),
    );
    let credential_mutation = lock_credential_mutation(credential_store, &credential_target)
        .map_err(|error| error.with_details(no_remote_mutation_details(scope, None)))?;
    if let Some(cancelled) = confirm_logout(&args, client_factory, &server, scope, confirmation)? {
        return Ok(cancelled);
    }
    preflight_credentials(credential_store, &credential_target)
        .map_err(|error| error.with_details(no_remote_mutation_details(scope, None)))?;
    let record = credential_mutation
        .load()
        .map_err(|error| {
            let credential_stored = matches!(
                &error,
                crate::credentials::CredentialError::Deserialize(_)
                    | crate::credentials::CredentialError::UnsupportedVersion(_)
                    | crate::credentials::CredentialError::InvalidRecord(_)
                    | crate::credentials::CredentialError::TimestampParse(_)
            )
            .then_some(true);
            map_credential_load_error(error)
                .with_details(no_remote_mutation_details(scope, credential_stored))
        })?
        .ok_or_else(|| credential_not_found(scope))?;
    let redactor = Redactor::with_secret(record.token());

    client
        .logout(&LogoutRequest { all: args.all }, record.token())
        .await
        .map_err(|error| map_client_error(error, &redactor, scope))?;

    let deletion = delete_completed_logout_credential(
        credential_mutation.as_ref(),
        &record,
        &redactor,
        scope,
        None,
    )?;

    Ok(success(
        server,
        client_factory
            .profile()
            .expect("authentication requires a named profile")
            .to_owned(),
        scope,
        true,
        deletion,
    ))
}

fn confirm_logout(
    args: &LogoutArgs,
    client_factory: &WekanClientFactory,
    server: &str,
    scope: LogoutScope,
    confirmation: &dyn ConfirmationProvider,
) -> Result<Option<CommandSuccess>, AppError> {
    let profile = client_factory
        .profile()
        .expect("authentication requires a named profile");
    let target = format!("profile `{profile}` on {server}");
    let prompt = match scope {
        LogoutScope::CurrentToken => format!(
            "Revoke the current Wekan login token and remove its local credential for {target}?"
        ),
        LogoutScope::AllTokens => format!(
            "Revoke all Wekan login tokens for {target}, including browser and other CLI sessions, and remove the local credential?"
        ),
        LogoutScope::LocalOnly => format!(
            "Remove the local credential for {target} without revoking its Wekan login token?"
        ),
    };
    let request = ConfirmationRequest::new(DestructiveOperation::AuthLogout, prompt);
    match confirm_or_skip(&args.confirmation, confirmation, &request)? {
        ConfirmationDecision::Proceed => Ok(None),
        ConfirmationDecision::Cancelled => Ok(Some(CommandSuccess::Cancelled(
            CancellationSuccess::new(DestructiveOperation::AuthLogout),
        ))),
    }
}

fn delete_completed_logout_credential(
    credential_mutation: &dyn CredentialMutation,
    record: &CredentialRecord,
    redactor: &Redactor<'_>,
    scope: LogoutScope,
    http_status: Option<u16>,
) -> Result<CredentialDeleteOutcome, AppError> {
    credential_mutation
        .delete_if_matches(record)
        .map_err(|error| {
            AppError::new(
                ErrorCode::CredentialStoreFailed,
                redactor.redact(&format!(
                    "Wekan completed logout, but the stored credential could not be removed: {error}"
                )),
                StableExitCode::Credential,
            )
            .with_details(ErrorDetails {
                http_status,
                logout_scope: Some(scope),
                remote_logout_completed: Some(Some(true)),
                ..ErrorDetails::default()
            })
        })
}

fn execute_local_only(
    client_factory: &WekanClientFactory,
    credential_mutation: &dyn CredentialMutation,
) -> Result<CommandSuccess, AppError> {
    let server = client_factory.server_identity().as_str().to_owned();
    let removed = credential_mutation.delete().map_err(|error| {
        AppError::new(
            ErrorCode::CredentialStoreFailed,
            format!("the stored credential could not be removed: {error}"),
            StableExitCode::Credential,
        )
        .with_details(ErrorDetails {
            logout_scope: Some(LogoutScope::LocalOnly),
            remote_logout_completed: Some(Some(false)),
            ..ErrorDetails::default()
        })
    })?;

    Ok(success(
        server,
        client_factory
            .profile()
            .expect("authentication requires a named profile")
            .to_owned(),
        LogoutScope::LocalOnly,
        false,
        if removed {
            CredentialDeleteOutcome::Removed
        } else {
            CredentialDeleteOutcome::Absent
        },
    ))
}

fn success(
    server: String,
    profile: String,
    logout_scope: LogoutScope,
    remote: bool,
    deletion: CredentialDeleteOutcome,
) -> CommandSuccess {
    CommandSuccess::Logout(LogoutSuccess {
        server,
        profile,
        logout_scope,
        remote_logout_completed: remote,
        credential_stored: deletion == CredentialDeleteOutcome::Mismatch,
        local_credential_removed: deletion == CredentialDeleteOutcome::Removed,
    })
}

fn credential_not_found(scope: LogoutScope) -> AppError {
    AppError::new(
        ErrorCode::CredentialNotFound,
        "no credential is stored for this Wekan server; run `wekan auth login` or `wekan auth register`",
        StableExitCode::Server,
    )
    .with_details(no_remote_mutation_details(scope, Some(false)))
}

fn no_remote_mutation_details(scope: LogoutScope, credential_stored: Option<bool>) -> ErrorDetails {
    ErrorDetails {
        logout_scope: Some(scope),
        remote_logout_completed: Some(Some(false)),
        credential_stored,
        local_credential_removed: Some(false),
        ..ErrorDetails::default()
    }
}

fn map_client_error(error: ClientError, redactor: &Redactor<'_>, scope: LogoutScope) -> AppError {
    match error {
        ClientError::Build(_) => AppError::new(
            ErrorCode::InternalError,
            "the HTTP client could not be initialized",
            StableExitCode::Internal,
        )
        .with_details(no_remote_mutation_details(scope, Some(true))),
        ClientError::Transport(error) => AppError::new(
            ErrorCode::TransportError,
            redactor.redact(&format!(
                "the logout request could not be completed: {error}; its remote outcome may be unknown"
            )),
            StableExitCode::Transport,
        )
        .with_details(ErrorDetails {
            logout_scope: Some(scope),
            remote_logout_completed: Some(None),
            outcome_unknown: Some(true),
            credential_stored: Some(true),
            local_credential_removed: Some(false),
            ..ErrorDetails::default()
        }),
        ClientError::UnexpectedRedirect { status } => AppError::new(
            ErrorCode::UnexpectedRedirect,
            "the Wekan server returned a redirect; logout requests are not replayed and the remote outcome may be unknown",
            StableExitCode::Transport,
        )
        .with_details(ErrorDetails {
            http_status: Some(status.as_u16()),
            logout_scope: Some(scope),
            remote_logout_completed: Some(None),
            outcome_unknown: Some(true),
            credential_stored: Some(true),
            local_credential_removed: Some(false),
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
            scope,
        ),
        ClientError::ResponseBody {
            status,
            retry_after_seconds,
            ..
        } => response_body_error(
            "the server response body could not be read".to_owned(),
            status,
            retry_after_seconds,
            scope,
        ),
        ClientError::Protocol {
            message,
            success_status_received,
        } => {
            let mut details = protocol_error_details(success_status_received);
            details.logout_scope = Some(scope);
            details.remote_logout_completed = Some(if success_status_received {
                None
            } else {
                Some(false)
            });
            details.outcome_unknown = success_status_received.then_some(true);
            details.credential_stored = Some(true);
            details.local_credential_removed = Some(false);
            AppError::new(
                ErrorCode::ProtocolError,
                redactor.redact(&format!("invalid response from Wekan: {message}")),
                StableExitCode::Transport,
            )
            .with_details(details)
        }
        ClientError::EmbeddedProtocol {
            http_status,
            server_error,
            server_reason,
            server_message,
            server_error_type,
            server_is_client_safe,
        } => {
            let mut details = embedded_protocol_error_details(
                http_status,
                server_error,
                server_reason,
                server_message,
                server_error_type,
                server_is_client_safe,
                redactor,
            );
            details.logout_scope = Some(scope);
            details.remote_logout_completed = Some(None);
            details.outcome_unknown = Some(true);
            details.credential_stored = Some(true);
            details.local_credential_removed = Some(false);
            AppError::new(
                ErrorCode::ProtocolError,
                "Wekan returned an embedded logout error without statusCode; the remote outcome may be unknown and the stored credential was preserved",
                StableExitCode::Server,
            )
            .with_details(details)
        }
        ClientError::Server {
            status,
            server_error,
            server_reason,
            retry_after_seconds,
        } => {
            let authentication_rejected = status == StatusCode::UNAUTHORIZED;
            let mut details = server_error_details(
                status,
                server_error,
                server_reason,
                retry_after_seconds,
                redactor,
            );
            details.logout_scope = Some(scope);
            details.outcome_unknown = status.is_server_error().then_some(true);
            details.remote_logout_completed = Some(if status.is_server_error() {
                None
            } else {
                Some(false)
            });
            details.credential_stored = Some(true);
            details.local_credential_removed = Some(false);
            AppError::new(
                if authentication_rejected {
                    ErrorCode::AuthenticationRejected
                } else {
                    ErrorCode::ServerError
                },
                if authentication_rejected {
                    "the stored credential was rejected by Wekan and was preserved locally"
                } else if status.is_server_error() {
                    "the Wekan server returned an unexpected logout error; the remote outcome may be unknown and the stored credential was preserved"
                } else {
                    "the Wekan server rejected logout and the stored credential was preserved"
                },
                StableExitCode::Server,
            )
            .with_details(details)
        }
        ClientError::EmbeddedServer {
            http_status,
            wekan_status_code,
            server_error,
            server_reason,
        } => {
            let mut details = embedded_server_error_details(
                http_status,
                wekan_status_code,
                server_error,
                server_reason,
                redactor,
            );
            details.logout_scope = Some(scope);
            details.remote_logout_completed = Some(Some(false));
            details.credential_stored = Some(true);
            details.local_credential_removed = Some(false);
            AppError::new(
                ErrorCode::ServerError,
                "the Wekan server returned an unexpected embedded logout error",
                StableExitCode::Server,
            )
            .with_details(details)
        }
    }
}

fn response_body_error(
    message: String,
    status: StatusCode,
    retry_after_seconds: Option<u64>,
    scope: LogoutScope,
) -> AppError {
    let mut details = response_error_details(status, retry_after_seconds);
    details.logout_scope = Some(scope);
    details.credential_stored = Some(true);
    details.local_credential_removed = Some(false);
    if status == StatusCode::OK {
        details.remote_logout_completed = Some(None);
        details.outcome_unknown = Some(true);
        return AppError::new(ErrorCode::ProtocolError, message, StableExitCode::Transport)
            .with_details(details);
    }

    let authentication_rejected = status == StatusCode::UNAUTHORIZED;
    details.outcome_unknown = status.is_server_error().then_some(true);
    details.remote_logout_completed = Some(if status.is_server_error() {
        None
    } else {
        Some(false)
    });
    AppError::new(
        if authentication_rejected {
            ErrorCode::AuthenticationRejected
        } else {
            ErrorCode::ServerError
        },
        if authentication_rejected {
            "the stored credential was rejected by Wekan and was preserved locally".to_owned()
        } else if status.is_server_error() {
            format!("{message}; the remote outcome may be unknown and the stored credential was preserved")
        } else {
            message
        },
        StableExitCode::Server,
    )
    .with_details(details)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            Arc, Condvar, Mutex,
            atomic::{AtomicUsize, Ordering},
            mpsc,
        },
        thread,
        time::Duration as StdDuration,
    };

    use clap::Parser;
    use secrecy::{ExposeSecret, SecretString};
    use serde_json::json;
    use time::{Duration, OffsetDateTime};
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_json, header, method, path},
    };

    use super::{LogoutArgs, execute as execute_with_confirmation};
    use crate::{
        cli::Cli,
        client::WekanClientFactory,
        command_result::{CommandSuccess, DestructiveOperation, LogoutScope},
        commands::{RootCommand, auth::AuthCommand},
        credentials::{
            CredentialCreateOutcome, CredentialDeleteOutcome, CredentialError, CredentialMutation,
            CredentialRecord, CredentialStore, CredentialTarget,
        },
        error::ErrorCode,
        exit_code::StableExitCode,
        input::{
            ConfirmationArgs, ConfirmationProvider, ConfirmationRequest, FakeConfirmationProvider,
        },
        output::{OutputFormat, render_error},
    };

    async fn execute(
        args: LogoutArgs,
        client_factory: &WekanClientFactory,
        credential_store: &dyn CredentialStore,
    ) -> Result<CommandSuccess, crate::error::AppError> {
        execute_with_confirmation(
            args,
            client_factory,
            credential_store,
            &FakeConfirmationProvider::accepting(),
        )
        .await
    }

    struct BlockingConfirmationProvider {
        entered: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    impl ConfirmationProvider for BlockingConfirmationProvider {
        fn confirm(&self, _request: &ConfirmationRequest) -> Result<bool, crate::error::AppError> {
            self.entered.send(()).unwrap();
            self.release.lock().unwrap().recv().unwrap();
            Ok(true)
        }
    }

    struct StoredCredential {
        account: String,
        user_id: String,
        token: String,
        token_expires: OffsetDateTime,
    }

    #[derive(Default)]
    struct FakeCredentialStore {
        credential: Mutex<Option<StoredCredential>>,
        mutation_locked: Mutex<bool>,
        mutation_released: Condvar,
        available: bool,
        panic_on_load: bool,
        load_error: Mutex<Option<CredentialError>>,
        delete_error: Mutex<Option<CredentialError>>,
        replacement_before_delete: Mutex<Option<StoredCredential>>,
        loads: AtomicUsize,
        deletes: AtomicUsize,
    }

    impl FakeCredentialStore {
        fn with_record(_server: &MockServer, token_expires: OffsetDateTime) -> Self {
            Self {
                credential: Mutex::new(Some(StoredCredential {
                    account: "profile:test-store:default".to_owned(),
                    user_id: "user-1".to_owned(),
                    token: "logout-token".to_owned(),
                    token_expires,
                })),
                available: true,
                ..Self::default()
            }
        }

        fn local_only(present: bool) -> Self {
            Self::local_only_account(present.then_some("profile:test-store:default"))
        }

        fn local_only_account(account: Option<&str>) -> Self {
            Self {
                credential: Mutex::new(account.map(|account| StoredCredential {
                    account: account.to_owned(),
                    user_id: "unreadable-record".to_owned(),
                    token: "not-decoded".to_owned(),
                    token_expires: OffsetDateTime::UNIX_EPOCH,
                })),
                available: true,
                panic_on_load: true,
                ..Self::default()
            }
        }

        fn has_credential(&self) -> bool {
            self.credential.lock().unwrap().is_some()
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

        fn exists(&self, target: &CredentialTarget) -> Result<bool, CredentialError> {
            Ok(self
                .credential
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|credential| credential.account == target.account()))
        }

        fn load(
            &self,
            target: &CredentialTarget,
        ) -> Result<Option<CredentialRecord>, CredentialError> {
            assert!(
                !self.panic_on_load,
                "local-only logout must never load credentials"
            );
            self.loads.fetch_add(1, Ordering::SeqCst);
            if let Some(error) = self.load_error.lock().unwrap().take() {
                return Err(error);
            }
            Ok(self
                .credential
                .lock()
                .unwrap()
                .as_ref()
                .filter(|credential| credential.account == target.account())
                .map(|credential| {
                    CredentialRecord::new(
                        target.server_url().to_owned(),
                        credential.user_id.clone(),
                        SecretString::from(credential.token.clone()),
                        credential.token_expires,
                    )
                }))
        }

        fn create(
            &self,
            _target: &CredentialTarget,
            _record: &CredentialRecord,
        ) -> Result<CredentialCreateOutcome, CredentialError> {
            panic!("logout must never save credentials")
        }

        fn delete(&self, target: &CredentialTarget) -> Result<bool, CredentialError> {
            self.deletes.fetch_add(1, Ordering::SeqCst);
            if let Some(error) = self.delete_error.lock().unwrap().take() {
                return Err(error);
            }
            let mut credential = self.credential.lock().unwrap();
            if credential
                .as_ref()
                .is_some_and(|credential| credential.account == target.account())
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
            self.deletes.fetch_add(1, Ordering::SeqCst);
            if let Some(error) = self.delete_error.lock().unwrap().take() {
                return Err(error);
            }
            let mut credential = self.credential.lock().unwrap();
            if let Some(replacement) = self.replacement_before_delete.lock().unwrap().take() {
                *credential = Some(replacement);
            }
            let Some(current) = credential.as_ref() else {
                return Ok(CredentialDeleteOutcome::Absent);
            };
            if current.account != target.account() {
                return Ok(CredentialDeleteOutcome::Absent);
            }
            let current = CredentialRecord::new(
                target.server_url().to_owned(),
                current.user_id.clone(),
                SecretString::from(current.token.clone()),
                current.token_expires,
            );
            if !current.matches(expected) {
                return Ok(CredentialDeleteOutcome::Mismatch);
            }
            credential.take();
            Ok(CredentialDeleteOutcome::Removed)
        }

        fn lock_mutation(
            &self,
            target: &CredentialTarget,
        ) -> Result<Box<dyn CredentialMutation + Send + '_>, CredentialError> {
            let mut locked = self.mutation_locked.lock().unwrap();
            while *locked {
                locked = self.mutation_released.wait(locked).unwrap();
            }
            *locked = true;
            drop(locked);
            Ok(Box::new(FakeCredentialMutation {
                store: self,
                target: target.clone(),
                _lock: FakeMutationLock { store: self },
            }))
        }
    }

    struct FakeMutationLock<'a> {
        store: &'a FakeCredentialStore,
    }

    impl Drop for FakeMutationLock<'_> {
        fn drop(&mut self) {
            let mut locked = self.store.mutation_locked.lock().unwrap();
            *locked = false;
            self.store.mutation_released.notify_one();
        }
    }

    struct FakeCredentialMutation<'a> {
        store: &'a FakeCredentialStore,
        target: CredentialTarget,
        _lock: FakeMutationLock<'a>,
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

    fn args(all: bool) -> LogoutArgs {
        LogoutArgs {
            all,
            local_only: false,
            confirmation: ConfirmationArgs::assume_yes(),
        }
    }

    fn factory(server: &MockServer) -> WekanClientFactory {
        WekanClientFactory::new(Some(server.uri()), false)
    }

    async fn mount_success(server: &MockServer, all: bool) {
        Mock::given(method("POST"))
            .and(path("/users/logout"))
            .and(header("authorization", "Bearer logout-token"))
            .and(body_json(json!({ "all": all })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "message": "logout complete"
            })))
            .expect(1)
            .mount(server)
            .await;
    }

    #[test]
    fn logout_flags_are_mutually_exclusive() {
        let current = Cli::try_parse_from([
            "wekan",
            "--server",
            "https://wekan.example",
            "auth",
            "logout",
        ])
        .unwrap();
        let RootCommand::Auth(auth) = current.command else {
            panic!("expected auth command")
        };
        let AuthCommand::Logout(args) = auth.command else {
            panic!("expected logout")
        };
        assert!(!args.all);
        assert!(!args.local_only);
        assert!(!args.confirmation.yes());

        let all = Cli::try_parse_from([
            "wekan",
            "--server",
            "https://wekan.example",
            "auth",
            "logout",
            "--all",
            "--yes",
        ])
        .unwrap();
        let RootCommand::Auth(auth) = all.command else {
            panic!("expected auth command")
        };
        let AuthCommand::Logout(args) = auth.command else {
            panic!("expected logout")
        };
        assert!(args.all);
        assert!(!args.local_only);
        assert!(args.confirmation.yes());

        assert!(
            Cli::try_parse_from([
                "wekan",
                "--server",
                "https://wekan.example",
                "auth",
                "logout",
                "--all",
                "--local-only",
            ])
            .is_err()
        );
    }

    #[tokio::test]
    async fn declining_remote_logout_preserves_credentials_and_skips_http_for_each_scope() {
        for (all, warning) in [
            (false, "current Wekan login token"),
            (true, "including browser and other CLI sessions"),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/users/logout"))
                .respond_with(ResponseTemplate::new(500))
                .expect(0)
                .mount(&server)
                .await;
            let store = FakeCredentialStore::with_record(
                &server,
                OffsetDateTime::now_utc() + Duration::days(1),
            );
            let confirmation = FakeConfirmationProvider::declining();

            let success = execute_with_confirmation(
                LogoutArgs {
                    all,
                    local_only: false,
                    confirmation: ConfirmationArgs::default(),
                },
                &factory(&server),
                &store,
                &confirmation,
            )
            .await
            .unwrap();

            let CommandSuccess::Cancelled(cancelled) = success else {
                panic!("expected cancellation")
            };
            assert_eq!(cancelled.operation, DestructiveOperation::AuthLogout);
            assert!(store.has_credential());
            assert_eq!(store.loads.load(Ordering::SeqCst), 0);
            assert_eq!(store.deletes.load(Ordering::SeqCst), 0);
            let requests = confirmation.requests();
            assert_eq!(requests.len(), 1);
            assert!(requests[0].prompt().contains(warning));
        }
    }

    #[tokio::test]
    async fn declining_or_failing_local_only_confirmation_never_touches_the_vault() {
        for (confirmation, expected_error) in [
            (FakeConfirmationProvider::declining(), false),
            (
                FakeConfirmationProvider::failing(crate::error::AppError::invalid_input(
                    "test confirmation unavailable",
                )),
                true,
            ),
        ] {
            let store = FakeCredentialStore::local_only(true);
            let result = execute_with_confirmation(
                LogoutArgs {
                    all: false,
                    local_only: true,
                    confirmation: ConfirmationArgs::default(),
                },
                &WekanClientFactory::new(Some("https://wekan.example".to_owned()), false),
                &store,
                &confirmation,
            )
            .await;

            assert_eq!(store.loads.load(Ordering::SeqCst), 0);
            assert_eq!(store.deletes.load(Ordering::SeqCst), 0);
            assert!(store.has_credential());
            assert!(
                confirmation.requests()[0]
                    .prompt()
                    .contains("without revoking its Wekan login token")
            );
            if expected_error {
                assert_eq!(result.unwrap_err().code(), ErrorCode::InvalidInput);
            } else {
                assert!(matches!(result.unwrap(), CommandSuccess::Cancelled(_)));
            }
        }
    }

    #[tokio::test]
    async fn yes_bypasses_the_confirmation_provider() {
        let store = FakeCredentialStore::local_only(true);
        let confirmation = FakeConfirmationProvider::failing(
            crate::error::AppError::invalid_input("must not be called"),
        );

        let success = execute_with_confirmation(
            LogoutArgs {
                all: false,
                local_only: true,
                confirmation: ConfirmationArgs::assume_yes(),
            },
            &WekanClientFactory::new(Some("https://wekan.example".to_owned()), false),
            &store,
            &confirmation,
        )
        .await
        .unwrap();

        assert!(matches!(success, CommandSuccess::Logout(_)));
        assert!(confirmation.requests().is_empty());
        assert!(!store.has_credential());
    }

    #[tokio::test]
    async fn local_only_deletes_without_loading_or_contacting_wekan() {
        let store = FakeCredentialStore::local_only(true);
        let success = execute(
            LogoutArgs {
                all: false,
                local_only: true,
                confirmation: ConfirmationArgs::assume_yes(),
            },
            &WekanClientFactory::new(Some("https://wekan.example".to_owned()), false),
            &store,
        )
        .await
        .unwrap();

        let CommandSuccess::Logout(success) = success else {
            panic!("expected logout success")
        };
        assert_eq!(success.server, "https://wekan.example/");
        assert_eq!(success.logout_scope, LogoutScope::LocalOnly);
        assert!(!success.remote_logout_completed);
        assert!(!success.credential_stored);
        assert!(success.local_credential_removed);
        assert!(!store.has_credential());
        assert_eq!(store.loads.load(Ordering::SeqCst), 0);
        assert_eq!(store.deletes.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn local_only_named_logout_deletes_only_that_profile_without_network_http_opt_in() {
        let factory = WekanClientFactory::for_profile(
            "http://wekan.example".to_owned(),
            "work".to_owned(),
            false,
        );

        let other_profile =
            FakeCredentialStore::local_only_account(Some("profile:test-store:personal"));
        let CommandSuccess::Logout(unchanged) = execute(
            LogoutArgs {
                all: false,
                local_only: true,
                confirmation: ConfirmationArgs::assume_yes(),
            },
            &factory,
            &other_profile,
        )
        .await
        .unwrap() else {
            panic!("expected logout success")
        };
        assert_eq!(unchanged.profile, "work");
        assert!(!unchanged.local_credential_removed);
        assert!(other_profile.has_credential());

        let selected_profile =
            FakeCredentialStore::local_only_account(Some("profile:test-store:work"));
        let CommandSuccess::Logout(removed) = execute(
            LogoutArgs {
                all: false,
                local_only: true,
                confirmation: ConfirmationArgs::assume_yes(),
            },
            &factory,
            &selected_profile,
        )
        .await
        .unwrap() else {
            panic!("expected logout success")
        };
        assert!(removed.local_credential_removed);
        assert!(!selected_profile.has_credential());
    }

    #[tokio::test]
    async fn local_only_is_idempotent_when_no_entry_exists() {
        let store = FakeCredentialStore::local_only(false);
        let success = execute(
            LogoutArgs {
                all: false,
                local_only: true,
                confirmation: ConfirmationArgs::assume_yes(),
            },
            &WekanClientFactory::new(Some("https://wekan.example".to_owned()), false),
            &store,
        )
        .await
        .unwrap();

        let CommandSuccess::Logout(success) = success else {
            panic!("expected logout success")
        };
        assert!(!success.credential_stored);
        assert!(!success.local_credential_removed);
        assert_eq!(store.loads.load(Ordering::SeqCst), 0);
        assert_eq!(store.deletes.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn local_only_delete_failure_does_not_claim_remote_logout() {
        let store = FakeCredentialStore::local_only(true);
        *store.delete_error.lock().unwrap() =
            Some(CredentialError::Delete("test delete failure".to_owned()));
        let error = execute(
            LogoutArgs {
                all: false,
                local_only: true,
                confirmation: ConfirmationArgs::assume_yes(),
            },
            &WekanClientFactory::new(Some("https://wekan.example".to_owned()), false),
            &store,
        )
        .await
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
        assert_eq!(error.exit_code(), StableExitCode::Credential);
        assert_eq!(error.details().logout_scope, Some(LogoutScope::LocalOnly));
        assert_eq!(error.details().remote_logout_completed, Some(Some(false)));
        assert!(store.has_credential());
    }

    #[tokio::test]
    async fn missing_remote_logout_credential_returns_without_http() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/users/logout"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let store = FakeCredentialStore {
            available: true,
            ..FakeCredentialStore::default()
        };

        let error = execute(args(false), &factory(&server), &store)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::CredentialNotFound);
        assert_eq!(store.deletes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn unreadable_remote_logout_credential_returns_without_http_or_deletion() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/users/logout"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let store = FakeCredentialStore {
            available: true,
            ..FakeCredentialStore::default()
        };
        *store.load_error.lock().unwrap() = Some(CredentialError::InvalidRecord("test invalid"));

        let error = execute(args(false), &factory(&server), &store)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
        assert_eq!(error.exit_code(), StableExitCode::Credential);
        assert_eq!(store.deletes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn credential_preflight_failure_stops_local_only_before_deletion() {
        let store = FakeCredentialStore::default();
        let error = execute(
            LogoutArgs {
                all: false,
                local_only: true,
                confirmation: ConfirmationArgs::assume_yes(),
            },
            &WekanClientFactory::new(Some("https://wekan.example".to_owned()), false),
            &store,
        )
        .await
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::CredentialStoreUnavailable);
        assert_eq!(error.exit_code(), StableExitCode::Credential);
        assert_eq!(store.loads.load(Ordering::SeqCst), 0);
        assert_eq!(store.deletes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn expired_credentials_are_still_revoked_and_deleted() {
        let server = MockServer::start().await;
        mount_success(&server, false).await;
        let store = FakeCredentialStore::with_record(
            &server,
            OffsetDateTime::now_utc() - Duration::days(1),
        );

        let success = execute(args(false), &factory(&server), &store)
            .await
            .unwrap();
        let CommandSuccess::Logout(success) = success else {
            panic!("expected logout success")
        };
        assert_eq!(success.logout_scope, LogoutScope::CurrentToken);
        assert!(success.remote_logout_completed);
        assert!(success.local_credential_removed);
        assert!(!store.has_credential());
    }

    #[tokio::test]
    async fn all_tokens_logout_uses_all_scope_and_deletes_after_success() {
        let server = MockServer::start().await;
        mount_success(&server, true).await;
        let store = FakeCredentialStore::with_record(
            &server,
            OffsetDateTime::now_utc() + Duration::days(1),
        );

        let success = execute(args(true), &factory(&server), &store)
            .await
            .unwrap();
        let CommandSuccess::Logout(success) = success else {
            panic!("expected logout success")
        };
        assert_eq!(success.logout_scope, LogoutScope::AllTokens);
        assert!(success.remote_logout_completed);
        assert!(success.local_credential_removed);
        assert!(!store.has_credential());
    }

    #[test]
    fn confirmation_holds_the_credential_lock_until_logout_finishes() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let server_url = format!("http://{}", listener.local_addr().unwrap());
        let account = format!("{server_url}/");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request).unwrap();

            let body = r#"{"message":"logout complete"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let store = Arc::new(FakeCredentialStore {
            credential: Mutex::new(Some(StoredCredential {
                account: "profile:test-store:default".to_owned(),
                user_id: "user-1".to_owned(),
                token: "logout-token".to_owned(),
                token_expires: OffsetDateTime::now_utc() + Duration::days(1),
            })),
            available: true,
            ..FakeCredentialStore::default()
        });
        let (confirmation_entered_sender, confirmation_entered_receiver) = mpsc::channel();
        let (release_confirmation_sender, release_confirmation_receiver) = mpsc::channel();
        let confirmation = BlockingConfirmationProvider {
            entered: confirmation_entered_sender,
            release: Mutex::new(release_confirmation_receiver),
        };
        let (logout_sender, logout_receiver) = mpsc::channel();
        let logout_store = Arc::clone(&store);
        let logout_server = server_url.clone();
        let logout = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let result = runtime.block_on(execute_with_confirmation(
                LogoutArgs {
                    all: true,
                    local_only: false,
                    confirmation: ConfirmationArgs::default(),
                },
                &WekanClientFactory::new(Some(logout_server), false),
                logout_store.as_ref(),
                &confirmation,
            ));
            logout_sender.send(result).unwrap();
        });

        confirmation_entered_receiver
            .recv_timeout(StdDuration::from_secs(1))
            .unwrap();
        let (contender_started_sender, contender_started_receiver) = mpsc::channel();
        let (contender_acquired_sender, contender_acquired_receiver) = mpsc::channel();
        let contender_store = Arc::clone(&store);
        let contender = thread::spawn(move || {
            contender_started_sender.send(()).unwrap();
            let target = CredentialTarget::profile_in_store("default", "test-store", account);
            let _mutation = contender_store.lock_mutation(&target).unwrap();
            contender_acquired_sender.send(()).unwrap();
        });
        contender_started_receiver
            .recv_timeout(StdDuration::from_secs(1))
            .unwrap();
        let acquired_before_confirmation = contender_acquired_receiver
            .recv_timeout(StdDuration::from_millis(250))
            .is_ok();

        release_confirmation_sender.send(()).unwrap();
        let result = logout_receiver
            .recv_timeout(StdDuration::from_secs(2))
            .unwrap()
            .unwrap();
        assert!(matches!(result, CommandSuccess::Logout(_)));
        if !acquired_before_confirmation {
            contender_acquired_receiver
                .recv_timeout(StdDuration::from_secs(1))
                .unwrap();
        }
        assert!(
            !acquired_before_confirmation,
            "another credential mutation acquired the target while confirmation was pending"
        );

        logout.join().unwrap();
        contender.join().unwrap();
        server.join().unwrap();
    }

    #[test]
    fn all_tokens_logout_serializes_the_remote_request_with_other_mutations() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server_url = format!("http://{address}");
        let account = format!("{server_url}/");
        let (request_started_sender, request_started_receiver) = mpsc::channel();
        let (respond_sender, respond_receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request).unwrap();
            request_started_sender.send(()).unwrap();
            respond_receiver.recv().unwrap();

            let body = r#"{"message":"logout complete"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let store = Arc::new(FakeCredentialStore {
            credential: Mutex::new(Some(StoredCredential {
                account: "profile:test-store:default".to_owned(),
                user_id: "user-1".to_owned(),
                token: "logout-token".to_owned(),
                token_expires: OffsetDateTime::now_utc() + Duration::days(1),
            })),
            available: true,
            ..FakeCredentialStore::default()
        });
        let (logout_sender, logout_receiver) = mpsc::channel();
        let logout_store = Arc::clone(&store);
        let logout_server = server_url.clone();
        let logout = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let result = runtime.block_on(execute(
                args(true),
                &WekanClientFactory::new(Some(logout_server), false),
                logout_store.as_ref(),
            ));
            logout_sender.send(result).unwrap();
        });

        request_started_receiver
            .recv_timeout(StdDuration::from_secs(1))
            .unwrap();
        let (contender_started_sender, contender_started_receiver) = mpsc::channel();
        let (contender_acquired_sender, contender_acquired_receiver) = mpsc::channel();
        let contender_store = Arc::clone(&store);
        let contender = thread::spawn(move || {
            contender_started_sender.send(()).unwrap();
            let target = CredentialTarget::profile_in_store("default", "test-store", account);
            let _mutation = contender_store.lock_mutation(&target).unwrap();
            contender_acquired_sender.send(()).unwrap();
        });
        contender_started_receiver
            .recv_timeout(StdDuration::from_secs(1))
            .unwrap();
        assert!(matches!(
            contender_acquired_receiver.recv_timeout(StdDuration::from_millis(250)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));

        respond_sender.send(()).unwrap();
        let result = logout_receiver
            .recv_timeout(StdDuration::from_secs(1))
            .unwrap()
            .unwrap();
        assert!(matches!(result, CommandSuccess::Logout(_)));
        contender_acquired_receiver
            .recv_timeout(StdDuration::from_secs(1))
            .unwrap();
        logout.join().unwrap();
        contender.join().unwrap();
        server.join().unwrap();
    }

    #[tokio::test]
    async fn authentication_rejection_preserves_the_credential() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/users/logout"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": "unauthorized",
                "reason": "logout-token was rejected"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let store = FakeCredentialStore::with_record(
            &server,
            OffsetDateTime::now_utc() + Duration::days(1),
        );

        let error = execute(args(false), &factory(&server), &store)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::AuthenticationRejected);
        assert_eq!(error.details().http_status, Some(401));
        assert_eq!(
            error.details().logout_scope,
            Some(LogoutScope::CurrentToken)
        );
        assert!(store.has_credential());
        assert_eq!(store.deletes.load(Ordering::SeqCst), 0);
        assert!(!error.message().contains("logout-token"));
        assert!(
            !error
                .details()
                .server_reason
                .as_deref()
                .unwrap()
                .contains("logout-token")
        );
    }

    #[tokio::test]
    async fn redirect_outcome_is_unknown_and_preserves_the_credential() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/users/logout"))
            .respond_with(ResponseTemplate::new(307).insert_header("location", "/other"))
            .expect(1)
            .mount(&server)
            .await;
        let store = FakeCredentialStore::with_record(
            &server,
            OffsetDateTime::now_utc() + Duration::days(1),
        );

        let error = execute(args(false), &factory(&server), &store)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::UnexpectedRedirect);
        assert_eq!(error.details().http_status, Some(307));
        assert_eq!(error.details().remote_logout_completed, Some(None));
        assert_eq!(error.details().outcome_unknown, Some(true));
        assert_eq!(error.details().credential_stored, Some(true));
        let rendered: serde_json::Value =
            serde_json::from_str(&render_error(OutputFormat::Json, &error)).unwrap();
        assert_eq!(
            rendered["error"]["details"]["remote_logout_completed"],
            serde_json::Value::Null
        );
        assert!(store.has_credential());
    }

    #[tokio::test]
    async fn server_failures_preserve_the_credential_and_report_uncertainty() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/users/logout"))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&server)
            .await;
        let store = FakeCredentialStore::with_record(
            &server,
            OffsetDateTime::now_utc() + Duration::days(1),
        );

        let error = execute(args(true), &factory(&server), &store)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::ServerError);
        assert_eq!(error.details().outcome_unknown, Some(true));
        assert_eq!(error.details().logout_scope, Some(LogoutScope::AllTokens));
        assert!(store.has_credential());
        assert_eq!(store.deletes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn unusable_http_200_preserves_the_credential_and_unknown_outcome() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/users/logout"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "wrong": true })))
            .expect(1)
            .mount(&server)
            .await;
        let store = FakeCredentialStore::with_record(
            &server,
            OffsetDateTime::now_utc() + Duration::days(1),
        );

        let error = execute(args(false), &factory(&server), &store)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::ProtocolError);
        assert_eq!(error.details().http_status, Some(200));
        assert_eq!(error.details().remote_logout_completed, Some(None));
        assert_eq!(error.details().outcome_unknown, Some(true));
        assert_eq!(error.details().credential_stored, Some(true));
        assert_eq!(error.details().local_credential_removed, Some(false));
        let rendered: serde_json::Value =
            serde_json::from_str(&render_error(OutputFormat::Json, &error)).unwrap();
        assert_eq!(
            rendered["error"]["details"]["remote_logout_completed"],
            serde_json::Value::Null
        );
        assert!(store.has_credential());
        assert_eq!(store.deletes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn oversized_http_200_preserves_the_credential_and_unknown_outcome() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/users/logout"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 1024 * 1024 + 1]))
            .expect(1)
            .mount(&server)
            .await;
        let store = FakeCredentialStore::with_record(
            &server,
            OffsetDateTime::now_utc() + Duration::days(1),
        );

        let error = execute(args(false), &factory(&server), &store)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::ProtocolError);
        assert_eq!(error.details().http_status, Some(200));
        assert_eq!(error.details().remote_logout_completed, Some(None));
        assert_eq!(error.details().outcome_unknown, Some(true));
        assert_eq!(error.details().credential_stored, Some(true));
        assert_eq!(error.details().local_credential_removed, Some(false));
        assert!(store.has_credential());
        assert_eq!(store.deletes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn truncated_http_200_preserves_the_credential_and_unknown_outcome() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server_url = format!("http://{address}");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request).unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{\"message\":",
                )
                .unwrap();
        });
        let store = FakeCredentialStore {
            credential: Mutex::new(Some(StoredCredential {
                account: "profile:test-store:default".to_owned(),
                user_id: "user-1".to_owned(),
                token: "logout-token".to_owned(),
                token_expires: OffsetDateTime::now_utc() + Duration::days(1),
            })),
            available: true,
            ..FakeCredentialStore::default()
        };

        let error = execute(
            args(false),
            &WekanClientFactory::new(Some(server_url), false),
            &store,
        )
        .await
        .unwrap_err();
        server.join().unwrap();

        assert_eq!(error.code(), ErrorCode::ProtocolError);
        assert_eq!(error.details().http_status, Some(200));
        assert_eq!(error.details().remote_logout_completed, Some(None));
        assert_eq!(error.details().outcome_unknown, Some(true));
        assert_eq!(error.details().credential_stored, Some(true));
        assert_eq!(error.details().local_credential_removed, Some(false));
        assert!(store.has_credential());
        assert_eq!(store.deletes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn deletion_failure_after_success_reports_the_remote_side_effect() {
        let server = MockServer::start().await;
        mount_success(&server, true).await;
        let store = FakeCredentialStore::with_record(
            &server,
            OffsetDateTime::now_utc() + Duration::days(1),
        );
        *store.delete_error.lock().unwrap() = Some(CredentialError::Delete(
            "logout-token could not be deleted".to_owned(),
        ));

        let error = execute(args(true), &factory(&server), &store)
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::CredentialStoreFailed);
        assert_eq!(error.details().logout_scope, Some(LogoutScope::AllTokens));
        assert_eq!(error.details().remote_logout_completed, Some(Some(true)));
        assert!(store.has_credential());
        assert!(!error.message().contains("logout-token"));
    }

    #[tokio::test]
    async fn remote_success_does_not_delete_a_concurrently_replaced_credential() {
        let server = MockServer::start().await;
        mount_success(&server, false).await;
        let store = FakeCredentialStore::with_record(
            &server,
            OffsetDateTime::now_utc() + Duration::days(1),
        );
        *store.replacement_before_delete.lock().unwrap() = Some(StoredCredential {
            account: "profile:test-store:default".to_owned(),
            user_id: "user-1".to_owned(),
            token: "new-login-token".to_owned(),
            token_expires: OffsetDateTime::now_utc() + Duration::days(2),
        });

        let success = execute(args(false), &factory(&server), &store)
            .await
            .unwrap();
        let CommandSuccess::Logout(success) = success else {
            panic!("expected logout success")
        };

        assert!(success.remote_logout_completed);
        assert!(success.credential_stored);
        assert!(!success.local_credential_removed);
        assert!(store.has_credential());
        assert_eq!(
            store.credential.lock().unwrap().as_ref().unwrap().token,
            "new-login-token"
        );
    }

    #[test]
    fn logout_request_debug_never_contains_the_token() {
        let request = crate::client::LogoutRequest { all: false };
        let token = SecretString::from("logout-token".to_owned());
        assert!(!format!("{request:?}").contains(token.expose_secret()));
    }
}
