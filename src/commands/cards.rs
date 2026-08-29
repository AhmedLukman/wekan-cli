mod create;
mod delete;
mod get;
mod list;
mod update;

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, config::MissingProfileResolution,
    credentials::CredentialStore, error::AppError, input::ConfirmationProvider,
};

#[derive(Debug, Args)]
pub struct CardArgs {
    #[command(subcommand)]
    pub command: CardCommand,
}

#[derive(Debug, Subcommand)]
pub enum CardCommand {
    /// List the cards returned by Wekan for a board list.
    List(list::ListArgs),

    /// Show one card by board, list, and card ID.
    Get(get::GetArgs),

    /// Create a card in a board list.
    Create(create::CreateArgs),

    /// Update one or more editable card fields.
    Update(update::UpdateArgs),

    /// Permanently delete a card and its child resources.
    Delete(delete::DeleteArgs),
}

impl CardCommand {
    pub(crate) const fn missing_profile_resolution(&self) -> MissingProfileResolution {
        MissingProfileResolution::Reject
    }
}

pub(crate) async fn dispatch(
    command: CardCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        CardCommand::List(args) => list::execute(args, client_factory, credential_store).await,
        CardCommand::Get(args) => get::execute(args, client_factory, credential_store).await,
        CardCommand::Create(args) => create::execute(args, client_factory, credential_store).await,
        CardCommand::Update(args) => update::execute(args, client_factory, credential_store).await,
        CardCommand::Delete(args) => {
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

fn rfc3339(value: &str) -> Result<String, String> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map(|_| value.to_owned())
        .map_err(|_| "value must be an RFC3339 date-time".to_owned())
}
