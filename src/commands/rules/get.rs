use crate::{
    client::WekanClientFactory,
    command_result::{CommandSuccess, RuleDetail},
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
    let d = context
        .client()
        .rule(
            &args.target.board_id,
            &args.target.rule_id,
            context.record().token(),
        )
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "rule lookup",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::RuleShown(Box::new(
        RuleDetail::from_document(args.target.board_id, d),
    )))
}
