use clap::{ArgGroup, Args};

use super::non_empty;
use crate::{
    client::{WekanClientFactory, checklists::UpdateChecklistRequest},
    command_result::{ChecklistCreateSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("updates").required(true).multiple(true).args(["title", "due_at", "clear_due_at"])))]
pub struct UpdateArgs {
    /// Wekan checklist ID.
    #[arg(value_parser = non_empty)]
    checklist_id: String,
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    board_id: String,
    /// Wekan card ID.
    #[arg(long = "card", value_parser = non_empty)]
    card_id: String,
    /// New checklist title.
    #[arg(long, value_parser = non_empty)]
    title: Option<String>,
    /// Checklist deadline as a Wekan date-time.
    #[arg(long, value_parser = deadline, conflicts_with = "clear_due_at")]
    due_at: Option<String>,
    /// Remove the checklist deadline.
    #[arg(long)]
    clear_due_at: bool,
}

pub(super) async fn execute(
    args: UpdateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let request = UpdateChecklistRequest {
        title: args.title,
        due_at: if args.clear_due_at {
            Some(None)
        } else {
            args.due_at.map(Some)
        },
    };
    let result = context
        .client()
        .update_checklist(
            &args.board_id,
            &args.card_id,
            &args.checklist_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "checklist update",
                "the requested Wekan checklist was not found on that card",
            )
        })?;
    Ok(CommandSuccess::ChecklistUpdated(ChecklistCreateSuccess {
        board_id: args.board_id,
        card_id: args.card_id,
        checklist_id: result.checklist_id,
    }))
}

fn deadline(value: &str) -> Result<String, String> {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .map(|_| value.to_owned())
        .map_err(|_| "expected an RFC 3339 date-time with a timezone".to_owned())
}
