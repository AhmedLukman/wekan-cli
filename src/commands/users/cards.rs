use clap::Args;

use crate::{
    client::{UserCardQuery, WekanClientFactory},
    command_result::{CommandSuccess, UserCard, UserCardsSuccess},
    commands::authenticated::AuthenticatedContext,
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::map_client_error;

#[derive(Debug, Args)]
pub struct CardsArgs {
    /// Only include cards with a due date.
    #[arg(long)]
    pub due: bool,

    /// Include cards due at or after this date accepted by Wekan.
    #[arg(long, allow_hyphen_values = true)]
    pub from: Option<String>,

    /// Include cards due at or before this date accepted by Wekan.
    #[arg(long, allow_hyphen_values = true)]
    pub to: Option<String>,
}

pub(super) async fn execute(
    args: CardsArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let cards = context
        .client()
        .user_cards(
            &UserCardQuery {
                due: args.due,
                from: args.from,
                to: args.to,
            },
            context.record().token(),
        )
        .await
        .map_err(|error| map_client_error(error, &redactor, "current-user cards"))?
        .into_iter()
        .map(|card| UserCard {
            card_id: card.card_id,
            title: card.title,
            board_id: card.board_id,
            swimlane_id: card.swimlane_id,
            list_id: card.list_id,
            due_at: card.due_at,
            start_at: card.start_at,
            end_at: card.end_at,
            members: card.members,
            assignees: card.assignees,
        })
        .collect();
    Ok(CommandSuccess::UserCards(UserCardsSuccess { cards }))
}
