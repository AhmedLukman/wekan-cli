mod create;
mod delete;
mod get;
mod list;
mod update;

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, config::MissingProfileResolution,
    credentials::CredentialStore, error::AppError, input::ConfirmationProvider,
};

#[derive(Debug, Args)]
pub struct SwimlaneArgs {
    #[command(subcommand)]
    pub command: SwimlaneCommand,
}

#[derive(Debug, Subcommand)]
pub enum SwimlaneCommand {
    /// List the swimlanes returned by Wekan for a board.
    List(list::ListArgs),

    /// Show one swimlane by board and swimlane ID.
    Get(get::GetArgs),

    /// Create a swimlane on a board.
    Create(create::CreateArgs),

    /// Update a swimlane title.
    Update(update::UpdateArgs),

    /// Permanently delete a swimlane and cascade according to Wekan behavior.
    Delete(delete::DeleteArgs),
}

impl SwimlaneCommand {
    pub(crate) const fn missing_profile_resolution(&self) -> MissingProfileResolution {
        MissingProfileResolution::Reject
    }
}

pub(crate) async fn dispatch(
    command: SwimlaneCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        SwimlaneCommand::List(args) => list::execute(args, client_factory, credential_store).await,
        SwimlaneCommand::Get(args) => get::execute(args, client_factory, credential_store).await,
        SwimlaneCommand::Create(args) => {
            create::execute(args, client_factory, credential_store).await
        }
        SwimlaneCommand::Update(args) => {
            update::execute(args, client_factory, credential_store).await
        }
        SwimlaneCommand::Delete(args) => {
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

fn finite_number(value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|_| "value must be a number".to_owned())?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("value must be finite".to_owned())
    }
}
