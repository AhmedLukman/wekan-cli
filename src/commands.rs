pub mod auth;

use clap::Subcommand;

use crate::{
    client::WekanClientFactory,
    credentials::{CredentialStore, PasswordProvider},
    error::AppError,
    output::CommandSuccess,
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
    password_provider: &dyn PasswordProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        RootCommand::Auth(args) => {
            auth::dispatch(
                args.command,
                client_factory,
                credential_store,
                password_provider,
            )
            .await
        }
    }
}
