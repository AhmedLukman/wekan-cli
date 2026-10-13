use super::non_empty;
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
#[command(next_help_heading = "Fields")]
pub struct ListArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,
}
pub(super) async fn execute(
    args: ListArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let fields = context
        .client()
        .board_custom_fields(&args.board_id, context.record().token())
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "custom-field collection",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::CustomFieldCollection(
        crate::command_result::CustomFieldCollectionSuccess {
            board_id: args.board_id,
            custom_fields: fields
                .into_iter()
                .map(|value| crate::command_result::CustomFieldSummary {
                    custom_field_id: value.custom_field_id,
                    name: value.name,
                    field_type: super::type_name(value.field_type),
                })
                .collect(),
        },
    ))
}
