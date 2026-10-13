mod create;
mod delete;
mod update;
use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, credentials::CredentialStore,
    error::AppError, input::ConfirmationProvider,
};
use clap::{Args, Subcommand};
#[derive(Debug, Args)]
pub struct CustomFieldItemArgs {
    #[command(subcommand)]
    pub command: CustomFieldItemCommand,
}
#[derive(Debug, Subcommand)]
pub enum CustomFieldItemCommand {
    /// Create custom-field dropdown items.
    Create(create::CreateArgs),
    /// Update custom-field dropdown items.
    Update(update::UpdateArgs),
    /// Permanently delete custom-field dropdown items.
    Delete(delete::DeleteArgs),
}
pub(crate) async fn dispatch(
    command: CustomFieldItemCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        CustomFieldItemCommand::Create(args) => create::execute(args, factory, store).await,
        CustomFieldItemCommand::Update(args) => update::execute(args, factory, store).await,
        CustomFieldItemCommand::Delete(args) => {
            delete::execute(args, factory, store, confirmation).await
        }
    }
}
use super::custom_fields::{exact_json, non_empty};
