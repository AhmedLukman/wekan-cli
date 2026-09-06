use clap::Args;
use reqwest::StatusCode;

use crate::commands::{
    authenticated::AuthenticatedContext,
    client_error::{
        embedded_protocol_error_details, embedded_server_error_details,
        protocol_diagnostic_details, response_error_details, server_error_details,
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
            diagnostic,
            message,
            success_status_received,
        } => AppError::new(
            ErrorCode::ProtocolError,
            redactor.redact(&format!("invalid response from Wekan: {message}")),
            StableExitCode::Transport,
        )
        .with_details(protocol_diagnostic_details(success_status_received, diagnostic, redactor)),
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
mod tests;
