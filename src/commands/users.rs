mod boards;
mod cards;
mod create;
mod current;
mod delete;
mod disable_login;
mod enable_login;
mod get;
mod list;
mod take_ownership;

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};
use reqwest::StatusCode;

use crate::{
    client::{ClientError, UserRecord, WekanClientFactory},
    command_result::{
        CommandSuccess, UserBoardMembership, UserDetail, UserEmail, UserOrganization, UserTeam,
    },
    config::MissingProfileResolution,
    credentials::{CredentialStore, SecretInputProvider},
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    input::ConfirmationProvider,
    redaction::Redactor,
};

use super::client_error::{
    embedded_protocol_error_details, embedded_server_error_details, protocol_error_details,
    response_error_details, server_error_details,
};

#[derive(Debug, Args)]
pub struct UserArgs {
    #[command(subcommand)]
    pub command: UserCommand,
}

#[derive(Debug, Subcommand)]
pub enum UserCommand {
    /// Show the currently authenticated Wekan user.
    Current(current::CurrentArgs),

    /// List cards related to the currently authenticated user.
    Cards(cards::CardsArgs),

    /// List Wekan users (site admin only).
    List(list::ListArgs),

    /// Show a Wekan user by ID or username (site admin only).
    Get(get::GetArgs),

    /// Create a Wekan user (site admin only).
    Create(create::CreateArgs),

    /// List active boards visible for a user ID.
    Boards(boards::BoardsArgs),

    /// Transfer boards administered by a user to the current user.
    TakeOwnership(take_ownership::TakeOwnershipArgs),

    /// Disable a user's login and clear their sessions.
    DisableLogin(disable_login::DisableLoginArgs),

    /// Re-enable a user's login.
    EnableLogin(enable_login::EnableLoginArgs),

    /// Delete a Wekan user.
    Delete(delete::DeleteArgs),
}

impl UserCommand {
    pub(crate) const fn missing_profile_resolution(&self) -> MissingProfileResolution {
        MissingProfileResolution::Reject
    }

    pub(crate) fn validate(&self) -> Result<(), AppError> {
        if let Self::Cards(args) = self {
            cards::validate_range(args.from.as_deref(), args.to.as_deref())?;
        }
        Ok(())
    }
}

pub(crate) async fn dispatch(
    command: UserCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        UserCommand::Current(args) => {
            current::execute(args, client_factory, credential_store).await
        }
        UserCommand::Cards(args) => cards::execute(args, client_factory, credential_store).await,
        UserCommand::List(args) => list::execute(args, client_factory, credential_store).await,
        UserCommand::Get(args) => get::execute(args, client_factory, credential_store).await,
        UserCommand::Create(args) => {
            create::execute(args, client_factory, credential_store, secret_input).await
        }
        UserCommand::Boards(args) => boards::execute(args, client_factory, credential_store).await,
        UserCommand::TakeOwnership(args) => {
            take_ownership::execute(args, client_factory, credential_store, confirmation).await
        }
        UserCommand::DisableLogin(args) => {
            disable_login::execute(args, client_factory, credential_store, confirmation).await
        }
        UserCommand::EnableLogin(args) => {
            enable_login::execute(args, client_factory, credential_store).await
        }
        UserCommand::Delete(args) => {
            delete::execute(args, client_factory, credential_store, confirmation).await
        }
    }
}

fn user_detail(user: UserRecord) -> UserDetail {
    UserDetail {
        user_id: user.user_id,
        username: user.username,
        full_name: user.profile.fullname,
        emails: user
            .emails
            .into_iter()
            .map(|email| UserEmail {
                address: email.address,
                verified: email.verified,
            })
            .collect(),
        is_admin: user.is_admin,
        login_disabled: user.login_disabled,
        authentication_method: user.authentication_method,
        created_at: user.created_at,
        modified_at: user.modified_at,
        last_connection_date: user.last_connection_date,
        organizations: user
            .orgs
            .into_iter()
            .map(|organization| UserOrganization {
                organization_id: organization.org_id,
                display_name: organization.org_display_name,
                is_admin: organization.is_admin,
            })
            .collect(),
        teams: user
            .teams
            .into_iter()
            .map(|team| UserTeam {
                team_id: team.team_id,
                display_name: team.team_display_name,
            })
            .collect(),
        boards: user
            .boards
            .into_iter()
            .map(|board| UserBoardMembership {
                board_id: board.board_id,
                is_active: board.is_active,
                is_admin: board.is_admin,
                is_no_comments: board.is_no_comments,
                is_comment_only: board.is_comment_only,
                is_worker: board.is_worker,
                is_normal_assigned_only: board.is_normal_assigned_only,
                is_comment_assigned_only: board.is_comment_assigned_only,
                is_read_only: board.is_read_only,
                is_read_assigned_only: board.is_read_assigned_only,
            })
            .collect(),
    }
}

fn non_empty(value: &str) -> Result<String, String> {
    if value.is_empty() {
        Err("value must not be empty".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

fn map_client_error(
    error: ClientError,
    redactor: &Redactor<'_>,
    operation: &str,
    mutation: bool,
) -> AppError {
    let (code, message, exit_code, mut details) = match error {
        ClientError::Build(_) => (
            ErrorCode::InternalError,
            format!("the {operation} HTTP client could not be initialized"),
            StableExitCode::Internal,
            ErrorDetails::default(),
        ),
        ClientError::Transport(error) => (
            ErrorCode::TransportError,
            format!("the {operation} request could not be completed: {error}"),
            StableExitCode::Transport,
            ErrorDetails::default(),
        ),
        ClientError::UnexpectedRedirect { status } => (
            ErrorCode::UnexpectedRedirect,
            format!("the {operation} request received an unexpected redirect"),
            StableExitCode::Transport,
            response_error_details(status, None),
        ),
        ClientError::ResponseTooLarge {
            limit_bytes,
            status,
            retry_after_seconds,
        } => response_body_error(
            format!("the {operation} response exceeded the {limit_bytes}-byte limit"),
            status,
            retry_after_seconds,
            operation,
        ),
        ClientError::ResponseBody {
            status,
            retry_after_seconds,
            ..
        } => response_body_error(
            format!("the {operation} response body could not be read"),
            status,
            retry_after_seconds,
            operation,
        ),
        ClientError::Protocol {
            message,
            success_status_received,
        } => (
            ErrorCode::ProtocolError,
            message,
            StableExitCode::Transport,
            protocol_error_details(success_status_received),
        ),
        ClientError::EmbeddedProtocol {
            http_status,
            server_error,
            server_reason,
            server_message,
            server_error_type,
            server_is_client_safe,
        } => (
            ErrorCode::ProtocolError,
            format!("the {operation} response contained a Wekan error without statusCode"),
            StableExitCode::Server,
            embedded_protocol_error_details(
                http_status,
                server_error,
                server_reason,
                server_message,
                server_error_type,
                server_is_client_safe,
                redactor,
            ),
        ),
        ClientError::Server {
            status,
            server_error,
            server_reason,
            retry_after_seconds,
        } => {
            let code = status_error_code(status.as_u16());
            let details = server_error_details(
                status,
                server_error,
                server_reason,
                retry_after_seconds,
                redactor,
            );
            (
                code,
                status_error_message(code, operation),
                StableExitCode::Server,
                details,
            )
        }
        ClientError::EmbeddedServer {
            http_status,
            wekan_status_code,
            server_error,
            server_reason,
        } => {
            let code = status_error_code(wekan_status_code);
            let details = embedded_server_error_details(
                http_status,
                wekan_status_code,
                server_error,
                server_reason,
                redactor,
            );
            (
                code,
                status_error_message(code, operation),
                StableExitCode::Server,
                details,
            )
        }
    };

    if mutation && mutation_outcome_is_unknown(code, &details) {
        details.outcome_unknown = Some(true);
    }
    AppError::new(code, redactor.redact(&message), exit_code).with_details(details)
}

fn response_body_error(
    message: String,
    status: StatusCode,
    retry_after_seconds: Option<u64>,
    operation: &str,
) -> (ErrorCode, String, StableExitCode, ErrorDetails) {
    if status == StatusCode::OK {
        return (
            ErrorCode::ProtocolError,
            message,
            StableExitCode::Transport,
            response_error_details(status, retry_after_seconds),
        );
    }

    let code = status_error_code(status.as_u16());
    (
        code,
        status_error_message(code, operation),
        StableExitCode::Server,
        response_error_details(status, retry_after_seconds),
    )
}

const fn status_error_code(status: u16) -> ErrorCode {
    match status {
        401 => ErrorCode::AuthenticationRejected,
        403 => ErrorCode::PermissionDenied,
        _ => ErrorCode::ServerError,
    }
}

fn status_error_message(code: ErrorCode, operation: &str) -> String {
    match code {
        ErrorCode::AuthenticationRejected => {
            "Wekan rejected the stored credential; remove it with `wekan auth logout --local-only --yes`, then log in again"
                .to_owned()
        }
        ErrorCode::PermissionDenied => format!("Wekan denied permission for {operation}"),
        _ => format!("Wekan rejected the {operation} request"),
    }
}

fn mutation_outcome_is_unknown(code: ErrorCode, details: &ErrorDetails) -> bool {
    match code {
        ErrorCode::AuthenticationRejected | ErrorCode::PermissionDenied => false,
        ErrorCode::ServerError => details
            .wekan_status_code
            .or(details.http_status)
            .is_none_or(|status| status >= StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
        ErrorCode::TransportError | ErrorCode::UnexpectedRedirect | ErrorCode::ProtocolError => {
            true
        }
        _ => false,
    }
}
