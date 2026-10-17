mod create;
mod delete;

use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    input::ConfirmationProvider,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct TokenArgs {
    #[command(subcommand)]
    pub command: TokenCommand,
}

#[derive(Debug, Subcommand)]
pub enum TokenCommand {
    /// Issue a token and print its secret in successful output (site admin only).
    Create(create::CreateArgs),
    /// Request deletion of one or all user tokens (site admin only).
    Delete(delete::DeleteArgs),
}

pub(super) async fn dispatch(
    command: TokenCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    secrets: &dyn SecretInputProvider,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        TokenCommand::Create(args) => create::execute(args, factory, store).await,
        TokenCommand::Delete(args) => {
            delete::execute(args, factory, store, secrets, confirmation).await
        }
    }
}
