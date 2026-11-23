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
    client::{ClientError, WekanClientFactory},
    command_result::CommandSuccess,
    commands::client_error::map_client_error_with_not_found,
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    input::ConfirmationProvider,
    redaction::Redactor,
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

fn map_target_error(error: ClientError, redactor: &Redactor<'_>, operation: &str) -> AppError {
    let native_error = match &error {
        ClientError::Server { server_error, .. } => server_error.as_deref(),
        _ => None,
    };
    let message = match native_error {
        Some("Board not found") => "the Wekan board was not found",
        Some("Integration not found") => "the integration was not found on that Wekan board",
        _ => "the integration or Wekan board was not found",
    };
    map_client_error_with_not_found(error, redactor, operation, message)
}

fn identifier(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || matches!(value, "." | "..") {
        Err("ID must not be empty, whitespace, or dot-only".to_owned())
    } else {
        Ok(value.to_owned())
    }
}
