pub mod auth;
pub(crate) mod authenticated;
pub mod boards;
pub mod cards;
pub(crate) mod client_error;
pub(crate) mod credential_ops;
pub mod lists;
pub mod profile;
pub mod swimlanes;
pub mod users;

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

    /// Inspect and administer Wekan users.
    User(users::UserArgs),

    /// Inspect and manage Wekan boards.
    Board(boards::BoardArgs),

    /// Inspect and manage lists on Wekan boards.
    List(lists::ListArgs),

    /// Inspect and manage cards in Wekan board lists.
    Card(Box<cards::CardArgs>),

    /// Inspect and manage swimlanes on Wekan boards.
    Swimlane(swimlanes::SwimlaneArgs),
}

pub(crate) enum PreparedCommand<'command, 'store> {
    Auth {
        command: auth::PreparedAuthCommand<'command, 'store>,
    },
    Profile {
        args: profile::ProfileArgs,
    },
    User {
        args: users::UserArgs,
        client_factory: &'command WekanClientFactory,
    },
    Board {
        args: boards::BoardArgs,
        client_factory: &'command WekanClientFactory,
    },
    List {
        args: lists::ListArgs,
        client_factory: &'command WekanClientFactory,
    },
    Card {
        args: Box<cards::CardArgs>,
        client_factory: &'command WekanClientFactory,
    },
    Swimlane {
        args: swimlanes::SwimlaneArgs,
        client_factory: &'command WekanClientFactory,
    },
}

pub(crate) async fn dispatch(
    command: PreparedCommand<'_, '_>,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
    profile_store: &dyn ProfileStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        PreparedCommand::Auth { command } => {
            auth::dispatch(command, credential_store, secret_input, confirmation).await
        }
        PreparedCommand::Profile { args } => {
            profile::dispatch(args.command, profile_store, credential_store, confirmation)
        }
        PreparedCommand::User {
            args,
            client_factory,
        } => {
            users::dispatch(
                args.command,
                client_factory,
                credential_store,
                secret_input,
                confirmation,
            )
            .await
        }
        PreparedCommand::Board {
            args,
            client_factory,
        } => boards::dispatch(args.command, client_factory, credential_store, confirmation).await,
        PreparedCommand::List {
            args,
            client_factory,
        } => lists::dispatch(args.command, client_factory, credential_store, confirmation).await,
        PreparedCommand::Card {
            args,
            client_factory,
        } => cards::dispatch(args.command, client_factory, credential_store, confirmation).await,
        PreparedCommand::Swimlane {
            args,
            client_factory,
        } => {
            swimlanes::dispatch(args.command, client_factory, credential_store, confirmation).await
        }
    }
}
