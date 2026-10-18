use clap::Args;

use super::non_empty;
use crate::{
    client::{ChecklistItemSortInput, CreateChecklistItemRequest, WekanClientFactory},
    command_result::{ChecklistItemCreateSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

#[derive(Debug, Args)]
#[command(next_help_heading = "Fields")]
pub struct CreateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan card ID.
    #[arg(long = "card", value_parser = non_empty, help_heading = "Target")]
    pub card_id: String,

    /// Wekan checklist ID.
    #[arg(long = "checklist", value_parser = non_empty, help_heading = "Target")]
    pub checklist_id: String,

    /// Item title, sent unchanged.
    #[arg(long)]
    pub title: String,

    /// Explicit finite sorting value; omit to append using Wekan's default.
    #[arg(long, value_parser = finite_number, allow_hyphen_values = true, conflicts_with = "sort_json")]
    pub sort: Option<f64>,

    /// Exact JSON number, string, or null for Wekan's sort coercion.
    #[arg(long, value_name = "JSON", value_parser = sort_json, allow_hyphen_values = true, conflicts_with = "sort")]
    pub sort_json: Option<ChecklistItemSortInput>,
}

pub(super) async fn execute(
    args: CreateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let request = CreateChecklistItemRequest {
        title: args.title,
        sort: args.sort_json.or_else(|| {
            args.sort.map(|value| {
                ChecklistItemSortInput::Number(
                    serde_json::Number::from_f64(value).expect("finite sort"),
                )
            })
        }),
    };
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let response = context
        .client()
        .create_checklist_item(
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
                "checklist item create",
                "the requested Wekan checklist item scope was not found",
            )
        })?;
    Ok(CommandSuccess::ChecklistItemCreated(
        ChecklistItemCreateSuccess {
            board_id: args.board_id,
            card_id: args.card_id,
            checklist_id: args.checklist_id,
            item_id: response.item_id,
        },
    ))
}

fn finite_number(value: &str) -> Result<f64, String> {
    let value = value
        .parse::<f64>()
        .map_err(|_| "value must be a number".to_owned())?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("value must be finite".to_owned())
    }
}

fn sort_json(value: &str) -> Result<ChecklistItemSortInput, String> {
    serde_json::from_str(value).map_err(|_| "expected a JSON number, string, or null".to_owned())
}
