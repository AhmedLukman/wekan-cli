use clap::Args;

use crate::commands::{
    client_error::{
        embedded_protocol_error_details, embedded_server_error_details, protocol_error_details,
        response_error_details, server_error_details,
    },
    credential_ops::{credential_target, lock_credential_mutation, preflight_credentials},
};
use crate::{
    client::{ClientError, LoginRequest},
    command_result::CommandSuccess,
    config::ResolvedTarget,
    credentials::{CredentialStore, LoginSecretMode, SecretInputProvider},
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    redaction::Redactor,
};

use super::{ensure_credential_absent, non_empty_identity, persist_session};

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
        .map_err(|error| error.with_session_created(false))?;
    let credential_mutation = lock_credential_mutation(credential_store, &credential_target)
        .map_err(|error| error.with_session_created(false))?;
    ensure_credential_absent(credential_mutation.as_ref(), target)
        .map_err(|error| error.with_session_created(false))?;

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

    let success = persist_session(target, credential_mutation.as_ref(), server_url, session)
        .map_err(|error| error.with_session_created(true))?;
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
        } => {
            let mut details = protocol_error_details(success_status_received);
            details.session_created = Some(success_status_received);
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
                "Wekan returned an embedded login error without statusCode; the remote outcome may be unknown",
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
                retry_after_seconds,
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
    let mut details = response_error_details(status, retry_after_seconds);
    if status == reqwest::StatusCode::OK {
        details.session_created = Some(true);
        return AppError::new(ErrorCode::ProtocolError, message, StableExitCode::Transport)
            .with_details(details);
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

    details.outcome_unknown = outcome_unknown;
    AppError::new(code, message, exit_code).with_details(details)
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
mod tests;
