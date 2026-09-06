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

use crate::{
    client::{UserRecord, WekanClientFactory},
    command_result::{
        CommandSuccess, UserBoardMembership, UserDetail, UserEmail, UserOrganization, UserTeam,
    },
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    input::ConfirmationProvider,
};

use super::client_error::map_client_error;

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
