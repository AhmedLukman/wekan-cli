use crate::{
    client::{WekanClientFactory, integrations::CreateIntegrationRequest},
    command_result::{CommandSuccess, IntegrationCreateSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
#[command(
    after_help = "Wekan validates the URL and creates an enabled outgoing webhook subscribed to all activities. Use integration update to configure other fields."
)]
pub struct CreateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = super::identifier)]
    board_id: String,
    /// Webhook URL, sent unchanged for Wekan to validate.
    #[arg(long)]
    url: String,
}

pub(super) async fn execute(
    args: CreateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let response = context
        .client()
        .create_integration(
            &args.board_id,
            &CreateIntegrationRequest { url: args.url },
            context.record().token(),
        )
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "integration creation",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::IntegrationCreated(
        IntegrationCreateSuccess {
            board_id: args.board_id,
            integration_id: response.integration_id,
        },
    ))
}
