use clap::Args;

use crate::{
    client::{CreateCardRequest, WekanClientFactory},
    command_result::{CardCreateSuccess, CommandSuccess},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::{non_empty, rfc3339, trimmed_non_empty};

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    pub board_id: String,

    /// Wekan list ID.
    #[arg(long = "list", value_parser = non_empty)]
    pub list_id: String,

    /// Card title.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub title: String,

    /// Wekan swimlane ID.
    #[arg(long, value_parser = non_empty)]
    pub swimlane_id: String,

    /// Card description.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub description: Option<String>,

    /// Member ID; may be repeated.
    #[arg(long = "member", value_parser = non_empty)]
    pub members: Vec<String>,

    /// Assignee ID; may be repeated.
    #[arg(long = "assignee", value_parser = non_empty)]
    pub assignees: Vec<String>,

    /// Received date-time in RFC3339 format.
    #[arg(long, value_parser = rfc3339)]
    pub received_at: Option<String>,

    /// Start date-time in RFC3339 format.
    #[arg(long, value_parser = rfc3339)]
    pub start_at: Option<String>,

    /// Due date-time in RFC3339 format.
    #[arg(long, value_parser = rfc3339)]
    pub due_at: Option<String>,

    /// End date-time in RFC3339 format.
    #[arg(long, value_parser = rfc3339)]
    pub end_at: Option<String>,
}

pub(super) async fn execute(
    args: CreateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let request = CreateCardRequest {
        title: args.title,
        swimlane_id: args.swimlane_id,
        description: args.description,
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
        .map_err(|error| map_client_error(error, &redactor, "card creation", true))?;
    Ok(CommandSuccess::CardCreated(CardCreateSuccess {
        board_id: args.board_id,
        list_id: args.list_id,
        card_id: created.card_id,
    }))
}
