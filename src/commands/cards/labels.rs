use crate::{
    client::WekanClientFactory,
    command_result::{CardLabelCollection, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
        custom_fields::non_empty,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct LabelArgs {
    #[command(subcommand)]
    pub command: LabelCommand,
}
#[derive(Debug, Subcommand)]
pub enum LabelCommand {
    /// List stored label IDs, preserving order and null elements.
    List(Target),
}
#[derive(Debug, Args)]
pub struct Target {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    board_id: String,
    /// Wekan list ID.
    #[arg(long = "list", value_parser = non_empty)]
    list_id: String,
    /// Wekan card ID.
    #[arg(long = "card", value_parser = non_empty)]
    card_id: String,
}
pub(super) async fn dispatch(
    command: LabelCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let LabelCommand::List(target) = command;
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let card = context
        .client()
        .card(
            &target.board_id,
            &target.list_id,
            &target.card_id,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "card label listing",
                "the requested Wekan card was not found in that board list",
            )
        })?;
    Ok(CommandSuccess::CardLabels(CardLabelCollection {
        board_id: target.board_id,
        list_id: target.list_id,
        card_id: card.card_id,
        label_ids: card.label_ids,
    }))
}
