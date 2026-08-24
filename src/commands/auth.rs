pub mod register;

use clap::{Args, Subcommand};

use crate::{
    client::WekanClientFactory,
    credentials::{CredentialStore, PasswordProvider},
    error::AppError,
    output::CommandSuccess,
};

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Register a Wekan account and securely store its login token.
    Register(register::RegisterArgs),
}

pub(crate) async fn dispatch(
    command: AuthCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    password_provider: &dyn PasswordProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        AuthCommand::Register(args) => {
            register::execute(args, client_factory, credential_store, password_provider).await
        }
    }
}
