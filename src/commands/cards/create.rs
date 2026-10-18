use clap::Args;

use crate::{
    client::{CreateCardRequest, WekanClientFactory},
    command_result::{CardCreateSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::{date_input, non_empty};

#[derive(Debug, Args)]
#[command(
    after_help = "Example:\n  wekan card create --board board-id --list list-id --title \"Plan next release\" --swimlane-id swimlane-id"
)]
#[command(next_help_heading = "Fields")]
pub struct CreateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan list ID.
    #[arg(long = "list", value_parser = non_empty, help_heading = "Target")]
    pub list_id: String,

    /// Card title; omit to use Wekan's empty default.
    #[arg(long, conflicts_with = "null_title")]
    pub title: Option<String>,

    /// Send an explicit null title.
    #[arg(long, conflicts_with = "title")]
    pub null_title: bool,

    /// Source card ID for linked-card creation. Wekan ignores other content fields.
    #[arg(long, value_parser = non_empty)]
    pub linked_id: Option<String>,

    /// Wekan swimlane ID.
    #[arg(long, value_parser = non_empty)]
    pub swimlane_id: String,

    /// Card description.
    #[arg(long, conflicts_with = "null_description")]
    pub description: Option<String>,

    /// Send an explicit null description.
    #[arg(long, conflicts_with = "description")]
    pub null_description: bool,

    /// Member ID; may be repeated.
    #[arg(long = "member", value_parser = non_empty)]
    pub members: Vec<String>,

    /// Assignee ID; may be repeated.
    #[arg(long = "assignee", value_parser = non_empty)]
    pub assignees: Vec<String>,

    /// Received date accepted by Wekan (timestamp, date-only, textual date or epoch milliseconds).
    #[arg(long, value_parser = date_input, allow_hyphen_values = true)]
    pub received_at: Option<crate::client::cards::CardDateInput>,

    /// Start date accepted by Wekan (timestamp, date-only, textual date or epoch milliseconds).
    #[arg(long, value_parser = date_input, allow_hyphen_values = true)]
    pub start_at: Option<crate::client::cards::CardDateInput>,

    /// Due date accepted by Wekan (timestamp, date-only, textual date or epoch milliseconds).
    #[arg(long, value_parser = date_input, allow_hyphen_values = true)]
    pub due_at: Option<crate::client::cards::CardDateInput>,

    /// End date accepted by Wekan (timestamp, date-only, textual date or epoch milliseconds).
    #[arg(long, value_parser = date_input, allow_hyphen_values = true)]
    pub end_at: Option<crate::client::cards::CardDateInput>,
}

pub(super) async fn execute(
    args: CreateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let request = CreateCardRequest {
        title: if args.null_title {
            Some(None)
        } else {
            args.title.map(Some)
        },
        linked_id: args.linked_id,
        swimlane_id: args.swimlane_id,
        description: if args.null_description {
            Some(None)
        } else {
            args.description.map(Some)
        },
        members: (!args.members.is_empty()).then_some(args.members),
        assignees: (!args.assignees.is_empty()).then_some(args.assignees),
        received_at: args.received_at,
        start_at: args.start_at,
        due_at: args.due_at,
        end_at: args.end_at,
    };
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let created = context
        .client()
        .create_card(
            &args.board_id,
            &args.list_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "card creation",
                "the linked source or requested card scope was not found",
            )
        })?;
    Ok(CommandSuccess::CardCreated(CardCreateSuccess {
        board_id: args.board_id,
        list_id: args.list_id,
        card_id: created.card_id,
    }))
}
