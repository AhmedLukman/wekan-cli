use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{CardCollectionSuccess, CardSummary, CommandSuccess},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use crate::commands::transfer::identifier;

#[derive(Debug, Args)]
#[command(
    after_help = "Examples:\n  wekan card list --board board-id --list list-id\n  wekan card list --board board-id --swimlane swimlane-id"
)]
pub struct ListArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = identifier, help_heading = "Target")]
    pub board_id: String,

    /// Wekan list ID.
    #[arg(long = "list", required_unless_present = "swimlane_id", conflicts_with = "swimlane_id", value_parser = identifier, help_heading = "Target")]
    pub list_id: Option<String>,

    /// Swimlane selector, mutually exclusive with --list.
    #[arg(long = "swimlane", value_parser = identifier, required_unless_present = "list_id", conflicts_with = "list_id", help_heading = "Target")]
    pub swimlane_id: Option<String>,
}

pub(super) async fn execute(
    args: ListArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    if let Some(swimlane_id) = args.swimlane_id {
        let cards = context
            .client()
            .swimlane_cards(&args.board_id, &swimlane_id, context.record().token())
            .await
            .map_err(|e| map_client_error(e, &redactor, "swimlane card collection"))?;
        return Ok(CommandSuccess::SwimlaneCards(
            crate::command_result::SwimlaneCardCollectionSuccess {
                board_id: args.board_id,
                swimlane_id,
                cards: cards
                    .into_iter()
                    .map(|card| crate::command_result::SwimlaneCardSummary {
                        card_id: card.card_id,
                        title: card.title,
                        description: card.description,
                        list_id: card.list_id,
                        received_at: card.received_at,
                        start_at: card.start_at,
                        due_at: card.due_at,
                        end_at: card.end_at,
                        assignees: card.assignees,
                        sort: card.sort,
                    })
                    .collect(),
            },
        ));
    }
    let list_id = args
        .list_id
        .expect("list or swimlane selector required by clap");
    let cards = context
        .client()
        .board_list_cards(&args.board_id, &list_id, context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "board list card collection"))?;
    Ok(CommandSuccess::CardCollection(CardCollectionSuccess {
        board_id: args.board_id,
        list_id,
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
