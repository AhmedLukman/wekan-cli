pub mod attachment_settings;
pub mod domains;
mod features;
mod input;
mod login_providers;
pub mod organizations;
mod problems;
pub mod settings;
pub mod teams;

use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    input::ConfirmationProvider,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct AdminArgs {
    #[command(subcommand)]
    pub command: AdminCommand,
}
#[derive(Debug, Subcommand)]
pub enum AdminCommand {
    /// Inspect and configure OAuth login providers (site admin).
    OauthProvider(login_providers::OauthProviderArgs),
    /// Configure email-code sign-in (site admin).
    Passwordless(login_providers::PasswordlessArgs),
    /// Inspect and acknowledge administrator problem reports.
    Problem(problems::ProblemArgs),
    /// Report primary-email domains and user counts (site admin).
    Domain(domains::DomainArgs),
    /// List organizations and configure their feature flags (site admin).
    Org(organizations::OrganizationArgs),
    /// List teams and configure their feature flags (site admin).
    Team(teams::TeamArgs),
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
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        AdminCommand::OauthProvider(args) => {
            login_providers::oauth(args.command, factory, store, secrets).await
        }
        AdminCommand::Passwordless(args) => {
            login_providers::passwordless(args.command, factory, store).await
        }
        AdminCommand::Problem(args) => problems::execute(args.command, factory, store).await,
        AdminCommand::Domain(args) => domains::execute(args.command, factory, store).await,
        AdminCommand::Org(args) => {
            organizations::execute(args.command, factory, store, confirmation).await
        }
        AdminCommand::Team(args) => {
            teams::execute(args.command, factory, store, confirmation).await
        }
        AdminCommand::Settings(args) => settings::execute(args.command, factory, store).await,
        AdminCommand::AttachmentSettings(args) => {
            attachment_settings::execute(args.command, factory, store, secrets).await
        }
    }
}
