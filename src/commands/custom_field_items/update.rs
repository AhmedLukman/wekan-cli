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
pub struct UpdateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,
    /// Wekan custom-field ID.
    #[arg(long = "custom-field", value_parser = non_empty, help_heading = "Target")]
    pub custom_field_id: String,
    /// Wekan dropdown item ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub item_id: String,
    /// New item name, sent unchanged; omitting it sends an empty update.
    #[arg(long)]
    pub name: Option<String>,
}
pub(super) async fn execute(
    args: UpdateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let submitted_fields = if args.name.is_some() {
        vec![crate::command_result::CustomFieldItemSubmittedField::Name]
    } else {
        vec![]
    };
    let request = crate::client::custom_fields::UpdateCustomFieldItemRequest { name: args.name };
    let response = context
        .client()
        .update_custom_field_item(
            &args.board_id,
            &args.custom_field_id,
            &args.item_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_mutation_client_error_with_not_found(
                error,
                &redactor,
                "dropdown item update",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::CustomFieldItemUpdated(
        crate::command_result::CustomFieldItemUpdateSuccess {
            board_id: args.board_id,
            custom_field_id: args.custom_field_id,
            item_id: response.id,
            submitted_fields,
        },
    ))
}
