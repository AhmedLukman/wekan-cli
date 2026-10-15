use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct GetArgs {
    #[command(flatten)]
    target: super::Target,
}

pub(super) async fn execute(
    args: GetArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let document = context
        .client()
        .integration(
            &args.target.board_id,
            &args.target.integration_id,
            context.record().token(),
        )
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "integration lookup",
                "the integration or Wekan board was not found",
            )
        })?
        .ok_or_else(super::missing_integration)?;
    Ok(CommandSuccess::IntegrationShown(document.into()))
}
