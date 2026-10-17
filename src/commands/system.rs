use clap::{Args, Subcommand};

use super::client_error::map_client_error;
use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, error::AppError,
    redaction::Redactor,
};

#[derive(Debug, Args)]
pub struct SystemArgs {
    #[command(subcommand)]
    pub command: SystemCommand,
}

#[derive(Debug, Subcommand)]
pub enum SystemCommand {
    /// Read public startup schema-upgrade status without credentials.
    SchemaUpgradeStatus,
}

pub(crate) async fn dispatch(
    command: SystemCommand,
    factory: &WekanClientFactory,
) -> Result<CommandSuccess, AppError> {
    match command {
        SystemCommand::SchemaUpgradeStatus => {
            let state = factory
                .create()
                .map_err(AppError::from)?
                .schema_upgrade_status()
                .await
                .map_err(|e| {
                    map_client_error(e, &Redactor::default(), "schema-upgrade status", false)
                })?;
            Ok(CommandSuccess::SchemaUpgradeStatus(state))
        }
    }
}
