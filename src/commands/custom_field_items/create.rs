use super::non_empty;
use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    commands::{
        authenticated::AuthenticatedContext, client_error::map_mutation_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;
#[derive(Debug, Args)]
#[command(next_help_heading = "Fields")]
pub struct CreateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,
    /// Wekan custom-field ID.
    #[arg(long = "custom-field", value_parser = non_empty, help_heading = "Target")]
    pub custom_field_id: String,
    /// Append item names in argument order.
    #[arg(long = "item", conflicts_with = "items")]
    pub item: Vec<String>,
    /// Exact items JSON; Wekan filters string entries and ignores non-array inputs.
    #[arg(long, value_name = "JSON", value_parser = super::exact_json, conflicts_with = "item")]
    pub items: Option<serde_json::Value>,
}
pub(super) async fn execute(
    args: CreateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let request = crate::client::custom_fields::CreateCustomFieldItemsRequest {
        items: args
            .items
            .or_else(|| (!args.item.is_empty()).then(|| serde_json::Value::from(args.item))),
    };
    let response = context
        .client()
        .create_custom_field_items(
            &args.board_id,
            &args.custom_field_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_mutation_client_error_with_not_found(
                error,
                &redactor,
                "dropdown item creation",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::CustomFieldItemsCreated(
        crate::command_result::CustomFieldItemCreateSuccess {
            board_id: args.board_id,
            custom_field_id: response.id,
        },
    ))
}
