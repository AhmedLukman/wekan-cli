use clap::Args;
use reqwest::StatusCode;

use crate::commands::{
    client_error::{
        embedded_protocol_error_details, embedded_server_error_details,
        protocol_diagnostic_details, response_error_details, server_error_details,
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
            diagnostic,
            message,
            success_status_received,
        } => {
            let mut details = protocol_diagnostic_details(success_status_received, diagnostic, redactor);
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
mod tests;
