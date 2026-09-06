use clap::Args;

use crate::commands::{
    client_error::{
        embedded_protocol_error_details, embedded_server_error_details,
        protocol_diagnostic_details, response_error_details, server_error_details,
    },
    credential_ops::{credential_target, lock_credential_mutation, preflight_credentials},
};
use crate::{
    client::{ClientError, RegisterRequest},
    command_result::CommandSuccess,
    config::ResolvedTarget,
    credentials::{CredentialStore, SecretInputProvider},
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    redaction::Redactor,
};

use super::{ensure_credential_absent, non_empty_identity, persist_session};

#[derive(Debug, Args)]
pub struct RegisterArgs {
    /// Username for the new account.
    #[arg(
        long,
        value_parser = non_empty_identity,
        required_unless_present = "email"
    )]
    pub username: Option<String>,

    /// Email address for the new account.
    #[arg(
        long,
        value_parser = non_empty_identity,
        required_unless_present = "username"
    )]
    pub email: Option<String>,

    /// Read the password from one line of standard input.
    #[arg(long)]
    pub password_stdin: bool,
}

pub(crate) async fn execute(
    args: RegisterArgs,
    target: &mut ResolvedTarget<'_>,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    let client = target.client_factory().create().map_err(AppError::from)?;
    let server_url = client.server().as_str().to_owned();
    let credential_target = credential_target(
        target.profile(),
        target.credential_namespace(),
        server_url.clone(),
    );

    preflight_credentials(credential_store, &credential_target)
        .map_err(|error| error.with_account_created(false))?;
    let credential_mutation = lock_credential_mutation(credential_store, &credential_target)
        .map_err(|error| error.with_account_created(false))?;
    ensure_credential_absent(credential_mutation.as_ref(), target)
        .map_err(|error| error.with_account_created(false))?;

    let password = secret_input.read_new_account_password(args.password_stdin)?;
    let request = RegisterRequest {
        username: args.username,
        email: args.email,
        password,
    };
    let redactor = Redactor::with_secret(&request.password);
    let session = client
        .register(&request)
        .await
        .map_err(|error| map_client_error(error, &redactor))?;

    let success = persist_session(target, credential_mutation.as_ref(), server_url, session)
        .map_err(|error| error.with_account_created(true))?;
    Ok(CommandSuccess::Registration(success))
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
            "the registration request failed; its remote outcome may be unknown, so check the server before retrying",
            StableExitCode::Transport,
        )
        .with_details(ErrorDetails {
            outcome_unknown: Some(true),
            ..ErrorDetails::default()
        }),
        ClientError::UnexpectedRedirect { status } => AppError::new(
            ErrorCode::UnexpectedRedirect,
            "the server redirected registration; configure the canonical Wekan URL because registration requests are never replayed",
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
            diagnostic,
            message,
            success_status_received,
        } => {
            let mut details = protocol_diagnostic_details(success_status_received, diagnostic, redactor);
            details.account_created = Some(success_status_received);
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
            details.outcome_unknown = Some(true);
            AppError::new(
                ErrorCode::ProtocolError,
                "Wekan returned an embedded registration error without statusCode; the remote outcome may be unknown",
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
            let (code, message, outcome_unknown) = match status.as_u16() {
                400 => (
                    ErrorCode::RegistrationRejected,
                    "Wekan returned a registration error; the remote outcome may be unknown, so check the server before retrying",
                    Some(true),
                ),
                403 => (
                    ErrorCode::RegistrationDisabled,
                    "registration is disabled on the Wekan server",
                    None,
                ),
                _ if status.is_server_error() => (
                    ErrorCode::ServerError,
                    "the Wekan server returned an unexpected error; the remote outcome may be unknown, so check the server before retrying",
                    Some(true),
                ),
                _ => (
                    ErrorCode::ServerError,
                    "the Wekan server returned an unexpected error",
                    None,
                ),
            };
            let mut details = server_error_details(
                status,
                server_error,
                server_reason,
                retry_after_seconds,
                redactor,
            );
            details.outcome_unknown = outcome_unknown;
            AppError::new(code, message, StableExitCode::Server).with_details(details)
        }
        ClientError::EmbeddedServer {
            http_status,
            wekan_status_code,
            server_error,
            server_reason,
        } => AppError::new(
            ErrorCode::ServerError,
            "the Wekan server returned an unexpected embedded registration error",
            StableExitCode::Server,
        )
        .with_details({
            let mut details = embedded_server_error_details(
                http_status,
                wekan_status_code,
                server_error,
                server_reason,
                redactor,
            );
            details.outcome_unknown = Some(true);
            details
        }),
    }
}

fn response_body_error(
    message: impl Into<String>,
    status: reqwest::StatusCode,
    retry_after_seconds: Option<u64>,
) -> AppError {
    let mut message = message.into();
    let mut details = response_error_details(status, retry_after_seconds);
    if status == reqwest::StatusCode::OK {
        details.account_created = Some(true);
        return AppError::new(ErrorCode::ProtocolError, message, StableExitCode::Transport)
            .with_details(details);
    }

    let (code, exit_code, outcome_unknown) = match status.as_u16() {
        400 => (
            ErrorCode::RegistrationRejected,
            StableExitCode::Server,
            Some(true),
        ),
        403 => (
            ErrorCode::RegistrationDisabled,
            StableExitCode::Server,
            None,
        ),
        _ if status.is_server_error() => {
            (ErrorCode::ServerError, StableExitCode::Server, Some(true))
        }
        _ => (ErrorCode::ServerError, StableExitCode::Server, None),
    };

    if outcome_unknown.is_some() {
        message
            .push_str("; the remote outcome may be unknown, so check the server before retrying");
    }

    let message = if status == reqwest::StatusCode::FORBIDDEN {
        "registration is disabled on the Wekan server".to_owned()
    } else {
        message
    };

    details.outcome_unknown = outcome_unknown;
    AppError::new(code, message, exit_code).with_details(details)
}

#[cfg(test)]
mod tests;
