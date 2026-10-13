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
    /// Field name, sent unchanged.
    #[arg(long)]
    pub name: String,
    /// Field type.
    #[arg(long = "type", value_parser = super::field_type)]
    pub field_type: crate::client::custom_fields::CustomFieldType,
    /// Typed Wekan settings JSON (defaults to {}).
    #[arg(long, value_name = "JSON", value_parser = super::settings, default_value = "{}")]
    pub settings: crate::client::custom_fields::CustomFieldSettings,
    /// show on card (explicit true or false).
    #[arg(long)]
    pub show_on_card: Option<bool>,
    /// automatically on card (explicit true or false).
    #[arg(long)]
    pub automatically_on_card: Option<bool>,
    /// show label on mini card (explicit true or false).
    #[arg(long)]
    pub show_label_on_mini_card: Option<bool>,
    /// show sum at top of list (explicit true or false).
    #[arg(long)]
    pub show_sum_at_top_of_list: Option<bool>,
}
pub(super) async fn execute(
    args: CreateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let request = crate::client::custom_fields::CreateCustomFieldRequest {
        name: args.name,
        field_type: args.field_type,
        settings: args.settings,
        show_on_card: args.show_on_card,
        automatically_on_card: args.automatically_on_card,
        show_label_on_mini_card: args.show_label_on_mini_card,
        show_sum_at_top_of_list: args.show_sum_at_top_of_list,
    };
    let response = context
        .client()
        .create_custom_field(&args.board_id, &request, context.record().token())
        .await
        .map_err(|error| {
            map_mutation_client_error_with_not_found(
                error,
                &redactor,
                "custom-field creation",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::CustomFieldCreated(
        crate::command_result::CustomFieldCreateSuccess {
            board_id: args.board_id,
            custom_field_id: response.id,
        },
    ))
}
