use crate::{
    client::WekanClientFactory,
    command_result::{CommandSuccess, DependencyUpdateSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_mutation_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
#[command(
    after_help = "Wekan removes empty metadata strings and stores nulls; dependency list normalizes both to defaults. No metadata sends {} and Wekan returns HTTP 500. A missing edge can be acknowledged without a change.\n\nExample:\n  wekan dependency update target-card-id --board board-id --card source-card-id --color '#2196f3'"
)]
pub struct UpdateArgs {
    #[command(flatten)]
    target: super::MutationTarget,
    #[command(flatten)]
    metadata: super::MetadataArgs,
}

pub(super) async fn execute(
    args: UpdateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let request = args.metadata.into_request();
    let submitted_fields = super::submitted_fields(&request);
    let response = context
        .client()
        .update_dependency(
            &args.target.board_id,
            &args.target.card_id,
            &args.target.target_card_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_mutation_client_error_with_not_found(
                error,
                &Redactor::with_secret(context.record().token()),
                "dependency update",
                "the Wekan board or source card was not found",
            )
        })?;
    Ok(CommandSuccess::DependencyUpdated(DependencyUpdateSuccess {
        board_id: args.target.board_id,
        card_id: response.card_id,
        target_card_id: args.target.target_card_id,
        submitted_fields,
        mutation_confirmed: false,
    }))
}
