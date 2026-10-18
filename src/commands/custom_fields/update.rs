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
pub struct UpdateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,
    /// Wekan custom-field ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub custom_field_id: String,
    /// New name, sent unchanged.
    #[arg(long)]
    pub name: Option<String>,
    /// New field type.
    #[arg(long = "type", value_parser = super::field_type)]
    pub field_type: Option<crate::client::custom_fields::CustomFieldType>,
    /// Replace settings with typed Wekan JSON.
    #[arg(long, value_name = "JSON", value_parser = super::settings)]
    pub settings: Option<crate::client::custom_fields::CustomFieldSettings>,
    /// show on card (explicit true or false).
    #[arg(long)]
    pub show_on_card: Option<bool>,
    /// automatically on card (explicit true or false).
    #[arg(long)]
    pub automatically_on_card: Option<bool>,
    /// always on card (explicit true or false).
    #[arg(long)]
    pub always_on_card: Option<bool>,
    /// show label on mini card (explicit true or false).
    #[arg(long)]
    pub show_label_on_mini_card: Option<bool>,
    /// show sum at top of list (explicit true or false).
    #[arg(long)]
    pub show_sum_at_top_of_list: Option<bool>,
}
pub(super) async fn execute(
    args: UpdateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let request = crate::client::custom_fields::UpdateCustomFieldRequest {
        name: args.name,
        field_type: args.field_type,
        settings: args.settings,
        show_on_card: args.show_on_card,
        automatically_on_card: args.automatically_on_card,
        always_on_card: args.always_on_card,
        show_label_on_mini_card: args.show_label_on_mini_card,
        show_sum_at_top_of_list: args.show_sum_at_top_of_list,
    };
    let mut submitted_fields = Vec::new();
    if request.name.is_some() {
        submitted_fields.push(crate::command_result::CustomFieldSubmittedField::Name);
    }
    if request.field_type.is_some() {
        submitted_fields.push(crate::command_result::CustomFieldSubmittedField::Type);
    }
    if request.settings.is_some() {
        submitted_fields.push(crate::command_result::CustomFieldSubmittedField::Settings);
    }
    if request.show_on_card.is_some() {
        submitted_fields.push(crate::command_result::CustomFieldSubmittedField::ShowOnCard);
    }
    if request.automatically_on_card.is_some() {
        submitted_fields
            .push(crate::command_result::CustomFieldSubmittedField::AutomaticallyOnCard);
    }
    if request.always_on_card.is_some() {
        submitted_fields.push(crate::command_result::CustomFieldSubmittedField::AlwaysOnCard);
    }
    if request.show_label_on_mini_card.is_some() {
        submitted_fields
            .push(crate::command_result::CustomFieldSubmittedField::ShowLabelOnMiniCard);
    }
    if request.show_sum_at_top_of_list.is_some() {
        submitted_fields.push(crate::command_result::CustomFieldSubmittedField::ShowSumAtTopOfList);
    }
    let response = context
        .client()
        .update_custom_field(
            &args.board_id,
            &args.custom_field_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "custom-field update",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::CustomFieldUpdated(
        crate::command_result::CustomFieldUpdateSuccess {
            board_id: args.board_id,
            custom_field_id: response.id,
            submitted_fields,
        },
    ))
}
