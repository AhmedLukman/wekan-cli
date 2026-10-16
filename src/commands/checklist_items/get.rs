use clap::Args;

use crate::{
    client::{ChecklistItemDocument, WekanClientFactory},
    command_result::{ChecklistItemDetail, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
#[command(
    after_help = "Example:\n  wekan checklist item get item-id --board board-id --card card-id --checklist checklist-id"
)]
pub struct GetArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan card ID.
    #[arg(long = "card", value_parser = non_empty, help_heading = "Target")]
    pub card_id: String,

    /// Wekan checklist ID.
    #[arg(long = "checklist", value_parser = non_empty, help_heading = "Target")]
    pub checklist_id: String,

    /// Wekan checklist item ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub item_id: String,
}

pub(super) async fn execute(
    args: GetArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let checklist_item = context
        .client()
        .checklist_item(
            &args.board_id,
            &args.card_id,
            &args.checklist_id,
            &args.item_id,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "checklist item lookup",
                "the requested Wekan checklist item was not found on that card",
            )
        })?;
    Ok(CommandSuccess::ChecklistItemShown(checklist_item_detail(
        checklist_item,
    )))
}

fn checklist_item_detail(document: ChecklistItemDocument) -> ChecklistItemDetail {
    ChecklistItemDetail {
        item_id: document.item_id,
        title: document.title,
        sort: document.sort,
        is_finished: document.is_finished,
        checklist_id: document.checklist_id,
        card_id: document.card_id,
        board_id: document.board_id,
        user_id: document.user_id,
        created_at: document.created_at,
        modified_at: document.modified_at,
    }
}
