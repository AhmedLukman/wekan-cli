pub mod auth;
pub mod profile;

use clap::Subcommand;

use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    config::profiles::ProfileStore,
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    input::ConfirmationProvider,
};

#[derive(Debug, Subcommand)]
pub enum RootCommand {
    /// Authenticate with a Wekan server.
    Auth(auth::AuthArgs),

    /// Manage named local Wekan server profiles.
    Profile(profile::ProfileArgs),
}

pub(crate) enum PreparedCommand<'a> {
    Auth {
        args: auth::AuthArgs,
        client_factory: &'a WekanClientFactory,
    },
    Profile {
        args: profile::ProfileArgs,
    },
}

pub(crate) async fn dispatch(
    command: PreparedCommand<'_>,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
    profile_store: &dyn ProfileStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        PreparedCommand::Auth {
            args,
            client_factory,
        } => {
            auth::dispatch(
                args.command,
                client_factory,
                credential_store,
                secret_input,
                confirmation,
            )
            .await
        }
        PreparedCommand::Profile { args } => {
            profile::dispatch(args.command, profile_store, credential_store, confirmation)
        }
    }
}
