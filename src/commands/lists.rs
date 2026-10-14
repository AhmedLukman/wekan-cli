mod create;
mod delete;
mod get;
mod list;
mod transfer;
mod update;

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, credentials::CredentialStore,
    error::AppError, input::ConfirmationProvider,
};

#[derive(Debug, Args)]
pub struct ListArgs {
    #[command(subcommand)]
    pub command: ListCommand,
}

#[derive(Debug, Subcommand)]
pub enum ListCommand {
    /// Copy a list using Wekan's native transfer behavior.
    Copy(transfer::TransferArgs),
    /// Move or reposition a list; Wekan may retain the source or fail after partial changes.
    Move(transfer::TransferArgs),
    /// List the lists returned by Wekan for a board.
    List(list::ListArgs),

    /// Show one list by board and list ID.
    Get(get::GetArgs),

    /// Create a list on a board.
    Create(create::CreateArgs),

    /// Update one or more editable list fields.
    Update(update::UpdateArgs),

    /// Soft-delete a list and its live cards.
    Delete(delete::DeleteArgs),
}

pub(crate) async fn dispatch(
    command: ListCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        ListCommand::Copy(args) => {
            transfer::execute(args, false, client_factory, credential_store).await
        }
        ListCommand::Move(args) => {
            transfer::execute(args, true, client_factory, credential_store).await
        }
        ListCommand::List(args) => list::execute(args, client_factory, credential_store).await,
        ListCommand::Get(args) => get::execute(args, client_factory, credential_store).await,
        ListCommand::Create(args) => create::execute(args, client_factory, credential_store).await,
        ListCommand::Update(args) => update::execute(args, client_factory, credential_store).await,
        ListCommand::Delete(args) => {
            delete::execute(args, client_factory, credential_store, confirmation).await
        }
    }
}

fn non_empty(value: &str) -> Result<String, String> {
    trimmed_non_empty(value)
}

fn trimmed_non_empty(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err("value must not be empty or whitespace".to_owned())
    } else {
        Ok(value.to_owned())
    }
}
