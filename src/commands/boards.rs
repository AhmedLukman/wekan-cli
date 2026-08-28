mod count;
mod create;
mod delete;
mod get;
mod list;
mod rename;

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, config::MissingProfileResolution,
    credentials::CredentialStore, error::AppError, input::ConfirmationProvider,
};

#[derive(Debug, Args)]
pub struct BoardArgs {
    #[command(subcommand)]
    pub command: BoardCommand,
}

#[derive(Debug, Subcommand)]
pub enum BoardCommand {
    /// List active boards for the authenticated user, or all public boards.
    List(list::ListArgs),

    /// Count all private and public boards on the server.
    Count(count::CountArgs),

    /// Show a board by ID.
    Get(get::GetArgs),

    /// Create a board and its default swimlane.
    Create(create::CreateArgs),

    /// Rename a board.
    Rename(rename::RenameArgs),

    /// Permanently delete a board and all of its contents.
    Delete(delete::DeleteArgs),
}

impl BoardCommand {
    pub(crate) const fn missing_profile_resolution(&self) -> MissingProfileResolution {
        MissingProfileResolution::Reject
    }
}

pub(crate) async fn dispatch(
    command: BoardCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        BoardCommand::List(args) => list::execute(args, client_factory, credential_store).await,
        BoardCommand::Count(args) => count::execute(args, client_factory, credential_store).await,
        BoardCommand::Get(args) => get::execute(args, client_factory, credential_store).await,
        BoardCommand::Create(args) => create::execute(args, client_factory, credential_store).await,
        BoardCommand::Rename(args) => rename::execute(args, client_factory, credential_store).await,
        BoardCommand::Delete(args) => {
            delete::execute(args, client_factory, credential_store, confirmation).await
        }
    }
}

fn non_empty(value: &str) -> Result<String, String> {
    if value.is_empty() {
        Err("value must not be empty".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

fn trimmed_non_empty(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err("value must not be empty or whitespace".to_owned())
    } else {
        Ok(value.to_owned())
    }
}
