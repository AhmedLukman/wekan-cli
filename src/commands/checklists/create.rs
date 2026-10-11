use clap::Args;

use super::non_empty;
use crate::{
    client::{ChecklistItemsInput, CreateChecklistRequest, WekanClientFactory},
    command_result::{ChecklistCreateSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_mutation_client_error_with_not_found,
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

    /// Checklist title; omit to use Wekan's default. Text is sent unchanged.
    #[arg(long)]
    pub title: Option<String>,

    /// Initial item title; may be repeated.
    #[arg(long = "item", conflicts_with = "items")]
    pub item_titles: Vec<String>,

    /// Exact JSON string, string array, or null for the initial items.
    #[arg(long, value_name = "JSON", value_parser = items_json, conflicts_with = "item_titles")]
    pub items: Option<ChecklistItemsInput>,
}

pub(super) async fn execute(
    args: CreateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let request = CreateChecklistRequest {
        title: args.title,
        items: args.items.or_else(|| {
            (!args.item_titles.is_empty()).then_some(ChecklistItemsInput::Array(args.item_titles))
        }),
    };
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let response = context
        .client()
        .create_checklist(
            &args.board_id,
            &args.card_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_mutation_client_error_with_not_found(
                error,
                &redactor,
                "checklist create",
                "the requested Wekan checklist scope was not found",
            )
        })?;
    Ok(CommandSuccess::ChecklistCreated(ChecklistCreateSuccess {
        board_id: args.board_id,
        card_id: args.card_id,
        checklist_id: response.checklist_id,
    }))
}

fn items_json(value: &str) -> Result<ChecklistItemsInput, String> {
    serde_json::from_str(value)
        .map_err(|_| "expected a JSON string, string array, or null".to_owned())
}
