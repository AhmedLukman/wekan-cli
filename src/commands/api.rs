mod request;

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, credentials::CredentialStore,
    error::AppError, input::ConfirmationProvider,
};

#[derive(Debug, Args)]
pub struct ApiArgs {
    #[command(subcommand)]
    pub command: ApiCommand,
}

#[derive(Debug, Subcommand)]
pub enum ApiCommand {
    /// Send an arbitrary same-origin HTTP request.
    Request(request::RequestArgs),
}

pub(crate) async fn dispatch(
    command: ApiCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        ApiCommand::Request(args) => {
            request::execute(args, client_factory, credential_store, confirmation).await
        }
    }
}
