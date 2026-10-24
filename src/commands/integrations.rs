mod activities;
mod create;
mod delete;
mod get;
mod inbound_email;
mod list;
#[cfg(test)]
mod tests;
mod update;

use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    credentials::{CredentialStore, SecretInputProvider},
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::ConfirmationProvider,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct IntegrationArgs {
    #[command(subcommand)]
    pub command: IntegrationCommand,
}

#[derive(Debug, Subcommand)]
pub enum IntegrationCommand {
    InboundEmail(inbound_email::InboundEmailArgs),
    /// List a board's webhook integrations.
    List(list::ListArgs),
    /// Show an integration and its activity subscriptions; tokens are excluded.
    Get(get::GetArgs),
    /// Create an outgoing webhook using Wekan's defaults.
    Create(create::CreateArgs),
    /// Update integration fields; omitted fields stay unchanged.
    Update(update::UpdateArgs),
    /// Delete an integration, with confirmation or --yes.
    Delete(delete::DeleteArgs),
    /// Add or remove webhook activity subscriptions.
    Activity(activities::ActivityArgs),
}

pub(crate) async fn dispatch(
    command: IntegrationCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    secrets: &dyn SecretInputProvider,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        IntegrationCommand::InboundEmail(args) => {
            inbound_email::dispatch(args.command, factory, secrets).await
        }
        IntegrationCommand::List(args) => list::execute(args, factory, store).await,
        IntegrationCommand::Get(args) => get::execute(args, factory, store).await,
        IntegrationCommand::Create(args) => create::execute(args, factory, store).await,
        IntegrationCommand::Update(args) => update::execute(args, factory, store, secrets).await,
        IntegrationCommand::Delete(args) => {
            delete::execute(args, factory, store, confirmation).await
        }
        IntegrationCommand::Activity(args) => {
            activities::dispatch(args.command, factory, store).await
        }
    }
}

#[derive(Debug, Args)]
struct Target {
    /// Wekan integration ID.
    #[arg(value_parser = identifier, help_heading = "Target")]
    integration_id: String,
    /// Wekan board ID.
    #[arg(long = "board", value_parser = identifier, help_heading = "Target")]
    board_id: String,
}

fn identifier(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || matches!(value, "." | "..") {
        Err("ID must not be empty, whitespace, or dot-only".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

fn missing_integration() -> AppError {
    AppError::new(
        ErrorCode::NotFound,
        "the integration was not found on that Wekan board",
        StableExitCode::Server,
    )
    .with_details(crate::commands::client_error::response_error_details(
        reqwest::StatusCode::OK,
        None,
    ))
}
