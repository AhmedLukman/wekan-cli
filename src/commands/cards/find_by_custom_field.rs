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
pub struct FindByCustomFieldArgs {
    /// Wekan board ID.
    #[arg(long="board",value_parser=non_empty,help_heading="Target")]
    pub board_id: String,
    /// Wekan custom-field ID.
    #[arg(long="custom-field",value_parser=non_empty,help_heading="Target")]
    pub custom_field_id: String,
    /// Exact nonempty string path value; dot-only values are unsupported.
    #[arg(long, value_parser = query_value, help_heading = "Query")]
    pub value: String,
}

fn query_value(value: &str) -> Result<String, String> {
    crate::client::cards::validate_custom_field_query_value(value).map_err(str::to_owned)?;
    Ok(value.to_owned())
}
pub(super) async fn execute(
    args: FindByCustomFieldArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let cards = context
        .client()
        .cards_by_custom_field(
            &args.board_id,
            &args.custom_field_id,
            &args.value,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "custom-field card collection",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::CardCustomFieldQuery(
        crate::command_result::CardCustomFieldQuerySuccess {
            board_id: args.board_id,
            custom_field_id: args.custom_field_id,
            value: args.value,
            cards: cards.into_iter().map(super::get::card_detail).collect(),
        },
    ))
}
