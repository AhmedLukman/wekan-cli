pub mod auth;

use clap::Subcommand;

use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
};

#[derive(Debug, Subcommand)]
pub enum RootCommand {
    /// Authenticate with a Wekan server.
    Auth(auth::AuthArgs),
}

pub(crate) async fn dispatch(
    command: RootCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        RootCommand::Auth(args) => {
            auth::dispatch(args.command, client_factory, credential_store, secret_input).await
        }
    }
}
