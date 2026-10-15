use crate::{
    client::WekanClientFactory,
    command_result::{AdminDomainCollectionSuccess, CommandSuccess},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct DomainArgs {
    #[command(subcommand)]
    pub command: DomainCommand,
}
#[derive(Debug, Subcommand)]
pub enum DomainCommand {
    /// List primary-email domains and user counts in Wekan's returned order.
    List,
}
pub(super) async fn execute(
    command: DomainCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    match command {
        DomainCommand::List => {
            let context = AuthenticatedContext::load(factory, store)?;
            let domains = context
                .client()
                .admin_domains(context.record().token())
                .await
                .map_err(|e| {
                    map_client_error(
                        e,
                        &Redactor::with_secret(context.record().token()),
                        "admin domain list",
                        false,
                    )
                })?;
            Ok(CommandSuccess::AdminDomains(AdminDomainCollectionSuccess {
                domains,
            }))
        }
    }
}
