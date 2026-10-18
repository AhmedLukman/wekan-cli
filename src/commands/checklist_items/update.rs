use clap::Args;

use super::non_empty;
use crate::{
    client::{ChecklistItemFinishedInput, UpdateChecklistItemRequest, WekanClientFactory},
    command_result::{ChecklistItemSubmittedField, ChecklistItemUpdateSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

#[derive(Debug, Args)]
#[command(next_help_heading = "Fields")]
pub struct UpdateArgs {
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

    /// New title, sent unchanged.
    #[arg(long)]
    pub title: Option<String>,

    /// Whether the item is checked.
    #[arg(long, conflicts_with = "is_finished_json")]
    pub is_finished: Option<bool>,

    /// Exact JSON boolean or string for Wekan's completion coercion.
    #[arg(long, value_name = "JSON", value_parser = finished_json, conflicts_with = "is_finished")]
    pub is_finished_json: Option<ChecklistItemFinishedInput>,
}

pub(super) async fn execute(
    args: UpdateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let request = UpdateChecklistItemRequest {
        title: args.title,
        is_finished: args
            .is_finished_json
            .or_else(|| args.is_finished.map(ChecklistItemFinishedInput::Boolean)),
    };
    let mut submitted_fields = Vec::new();
    if request.title.is_some() {
        submitted_fields.push(ChecklistItemSubmittedField::Title);
    }
    if request.is_finished.is_some() {
        submitted_fields.push(ChecklistItemSubmittedField::IsFinished);
    }
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let response = context
        .client()
        .update_checklist_item(
            &args.board_id,
            &args.card_id,
            &args.checklist_id,
            &args.item_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "checklist item update",
                "the requested Wekan checklist item scope was not found",
            )
        })?;
    Ok(CommandSuccess::ChecklistItemUpdated(
        ChecklistItemUpdateSuccess {
            board_id: args.board_id,
            card_id: args.card_id,
            checklist_id: args.checklist_id,
            item_id: response.item_id,
            submitted_fields,
        },
    ))
}

fn finished_json(value: &str) -> Result<ChecklistItemFinishedInput, String> {
    serde_json::from_str(value).map_err(|_| "expected a JSON boolean or string".to_owned())
}
