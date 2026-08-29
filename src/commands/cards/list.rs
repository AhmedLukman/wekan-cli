use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{CardCollectionSuccess, CardSummary, CommandSuccess},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Wekan board ID.
    #[arg(value_parser = non_empty)]
    pub board_id: String,

    /// Wekan list ID.
    #[arg(value_parser = non_empty)]
    pub list_id: String,
}

pub(super) async fn execute(
    args: ListArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let cards = context
        .client()
        .board_list_cards(&args.board_id, &args.list_id, context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "board list card collection", false))?;
    Ok(CommandSuccess::CardCollection(CardCollectionSuccess {
        board_id: args.board_id,
        list_id: args.list_id,
        cards: cards
            .into_iter()
            .map(|card| CardSummary {
                card_id: card.card_id,
                title: card.title,
                description: card.description,
                swimlane_id: card.swimlane_id,
                received_at: card.received_at,
                start_at: card.start_at,
                due_at: card.due_at,
                end_at: card.end_at,
                assignees: card.assignees,
                sort: card.sort,
            })
            .collect(),
    }))
}
