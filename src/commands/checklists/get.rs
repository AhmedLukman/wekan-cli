use clap::Args;

use crate::{
    client::{ChecklistDocument, WekanClientFactory},
    command_result::{ChecklistDetail, CommandSuccess},
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
    after_help = "Example:\n  wekan checklist get checklist-id --board board-id --card card-id"
)]
pub struct GetArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan card ID.
    #[arg(long = "card", value_parser = non_empty, help_heading = "Target")]
    pub card_id: String,

    /// Wekan checklist ID.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub checklist_id: String,
}

pub(super) async fn execute(
    args: GetArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let checklist = context
        .client()
        .checklist(
            &args.board_id,
            &args.card_id,
            &args.checklist_id,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "checklist lookup",
                "the requested Wekan checklist was not found on that card",
            )
        })?;
    Ok(CommandSuccess::ChecklistShown(checklist_detail(checklist)))
}

pub(super) fn checklist_detail(document: ChecklistDocument) -> ChecklistDetail {
    ChecklistDetail {
        last_reset_at: document.last_reset_at,
        reset_interval: document.reset_interval,
        due_at: document.due_at,
        checklist_id: document.checklist_id,
        card_id: document.card_id,
        board_id: document.board_id,
        user_id: document.user_id,
        title: document.title,
        finished_at: document.finished_at,
        created_at: document.created_at,
        modified_at: document.modified_at,
        sort: document.sort,
        hide_checked_checklist_items: document.hide_checked_checklist_items,
        hide_all_checklist_items: document.hide_all_checklist_items,
        show_checklist_at_minicard: document.show_checklist_at_minicard,
        items: document
            .items
            .into_iter()
            .map(|item| crate::command_result::ChecklistItemSummary {
                due_at: item.due_at,
                item_id: item.item_id,
                title: item.title,
                is_finished: item.is_finished,
            })
            .collect(),
    }
}
