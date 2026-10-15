use crate::{
    client::WekanClientFactory,
    command_result::{CommandSuccess, RuleCollectionSuccess, RuleDetail},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = super::identifier)]
    board_id: String,
}
pub(super) async fn execute(
    args: ListArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let documents = context
        .client()
        .board_rules(&args.board_id, context.record().token())
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "rule listing",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::RuleCollection(RuleCollectionSuccess {
        rules: documents
            .into_iter()
            .map(|d| RuleDetail::from_document(args.board_id.clone(), d))
            .collect(),
        board_id: args.board_id,
    }))
}
