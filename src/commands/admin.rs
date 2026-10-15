pub mod attachment_settings;
mod input;
pub mod settings;

use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct AdminArgs {
    #[command(subcommand)]
    pub command: AdminCommand,
}
#[derive(Debug, Subcommand)]
pub enum AdminCommand {
    /// Read or update the global REST settings allowlist (site admin).
    Settings(settings::SettingsArgs),
    /// Configure attachment backends, restrictions, and migration settings (site admin).
    AttachmentSettings(attachment_settings::AttachmentSettingsArgs),
}
pub(crate) async fn dispatch(
    command: AdminCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    secrets: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        AdminCommand::Settings(args) => settings::execute(args.command, factory, store).await,
        AdminCommand::AttachmentSettings(args) => {
            attachment_settings::execute(args.command, factory, store, secrets).await
        }
    }
}
