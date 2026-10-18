use crate::commands::custom_fields::non_empty;
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
pub struct SetCustomFieldArgs {
    /// Wekan board ID.
    #[arg(long = "board",value_parser=non_empty,help_heading="Target")]
    pub board_id: String,
    /// Wekan list ID.
    #[arg(long = "list",value_parser=non_empty,help_heading="Target")]
    pub list_id: String,
    /// Wekan card ID.
    #[arg(value_parser=non_empty,help_heading="Target")]
    pub card_id: String,
    /// Existing custom-field entry ID.
    #[arg(long = "custom-field",value_parser=non_empty,help_heading="Target")]
    pub custom_field_id: String,
    /// Text value, sent unchanged; conflicts with --value-json.
    #[arg(long, conflicts_with = "value_json", help_heading = "Fields")]
    pub value: Option<String>,
    /// Exact JSON value; omission sends no value property.
    #[arg(long,value_name="JSON",value_parser=crate::commands::custom_fields::exact_json,conflicts_with="value",help_heading="Fields")]
    pub value_json: Option<serde_json::Value>,
}
pub(super) async fn execute(
    args: SetCustomFieldArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let request = crate::client::cards::SetCardCustomFieldRequest {
        value: args
            .value_json
            .or_else(|| args.value.map(serde_json::Value::String)),
    };
    let response = context
        .client()
        .set_card_custom_field(
            &args.board_id,
            &args.list_id,
            &args.card_id,
            &args.custom_field_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "card custom-field assignment",
                "the Wekan board or card scope was not found",
            )
        })?;
    Ok(CommandSuccess::CardCustomFieldSet(
        crate::command_result::CardCustomFieldSetSuccess {
            board_id: args.board_id,
            list_id: args.list_id,
            card_id: response.card_id,
            custom_field_id: args.custom_field_id,
            custom_fields: response
                .custom_fields
                .into_iter()
                .map(super::get::card_detail_custom_field)
                .collect(),
        },
    ))
}
