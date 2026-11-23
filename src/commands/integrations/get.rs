use crate::{
    client::WekanClientFactory, command_result::CommandSuccess,
    commands::authenticated::AuthenticatedContext, credentials::CredentialStore, error::AppError,
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
            super::map_target_error(
                e,
                &Redactor::with_secret(context.record().token()),
                "integration lookup",
            )
        })?;
    Ok(CommandSuccess::IntegrationShown(document.into()))
}
