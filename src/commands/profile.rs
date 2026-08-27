mod add;
mod list;
mod remove;
mod show;
mod update;
mod use_profile;

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};

pub use self::{
    add::AddArgs, list::ListArgs, remove::RemoveArgs, show::ShowArgs, update::UpdateArgs,
    use_profile::UseArgs,
};
use crate::{
    command_result::CommandSuccess,
    config::profiles::ProfileStore,
    credentials::{CredentialError, CredentialStore, CredentialTarget},
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    input::ConfirmationProvider,
};

#[derive(Debug, Args)]
pub struct ProfileArgs {
    #[command(subcommand)]
    pub command: ProfileCommand,
}

#[derive(Debug, Subcommand)]
pub enum ProfileCommand {
    /// Add a named Wekan server profile.
    Add(AddArgs),
    /// List configured Wekan server profiles.
    List(ListArgs),
    /// Show one configured Wekan server profile.
    Show(ShowArgs),
    /// Select the active Wekan server profile.
    Use(UseArgs),
    /// Change a profile's Wekan server URL.
    Update(UpdateArgs),
    /// Remove a Wekan server profile.
    Remove(RemoveArgs),
}

pub(crate) fn dispatch(
    command: ProfileCommand,
    profile_store: &dyn ProfileStore,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        ProfileCommand::Add(args) => {
            let name = args.name.clone();
            with_profile_context(name, add::execute(args, profile_store))
        }
        ProfileCommand::List(args) => list::execute(args, profile_store),
        ProfileCommand::Show(args) => {
            let name = args.name.clone();
            with_profile_context(name, show::execute(args, profile_store))
        }
        ProfileCommand::Use(args) => {
            let name = args.name.clone();
            with_profile_context(name, use_profile::execute(args, profile_store))
        }
        ProfileCommand::Update(args) => {
            let name = args.name.clone();
            with_profile_context(name, update::execute(args, profile_store, credential_store))
        }
        ProfileCommand::Remove(args) => {
            let name = args.name.clone();
            with_profile_context(
                name,
                remove::execute(args, profile_store, credential_store, confirmation),
            )
        }
    }
}

fn with_profile_context(
    name: String,
    result: Result<CommandSuccess, AppError>,
) -> Result<CommandSuccess, AppError> {
    result.map_err(|error| error.with_profile_context(Some(name)))
}

fn ensure_logged_out(
    credential_store: &dyn CredentialStore,
    name: &str,
    server: &str,
    credential_namespace: &str,
) -> Result<(), AppError> {
    let target = CredentialTarget::profile_in_store(name, credential_namespace, server.to_owned());
    match credential_store.load(&target) {
        Ok(None) => Ok(()),
        Ok(Some(_)) => Err(profile_error(
            ErrorCode::ProfileHasCredential,
            name,
            format!(
                "profile `{name}` has a stored credential; run `wekan --profile {name} auth logout` or `wekan --profile {name} auth logout --local-only` first"
            ),
        )),
        Err(CredentialError::Unavailable(message)) => Err(AppError::new(
            ErrorCode::CredentialStoreUnavailable,
            format!("the operating-system credential store is unavailable: {message}"),
            StableExitCode::Credential,
        )
        .with_profile_context(Some(name.to_owned()))),
        Err(error) => Err(AppError::new(
            ErrorCode::CredentialStoreFailed,
            format!("the stored credential could not be checked: {error}"),
            StableExitCode::Credential,
        )
        .with_profile_context(Some(name.to_owned()))),
    }
}

fn profile_not_found(name: &str) -> AppError {
    profile_error(
        ErrorCode::ProfileNotFound,
        name,
        format!("profile `{name}` does not exist"),
    )
}

fn profile_error(code: ErrorCode, name: &str, message: String) -> AppError {
    AppError::new(code, message, StableExitCode::Configuration).with_details(ErrorDetails {
        profile: Some(Some(name.to_owned())),
        ..ErrorDetails::default()
    })
}
