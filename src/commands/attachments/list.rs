use crate::{
    client::WekanClientFactory,
    command_result::{AttachmentCollectionSuccess, AttachmentScope, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = super::identifier)]
    pub board_id: String,
    /// Requested swimlane ID; requires --board. Ignored by pinned Wekan.
    #[arg(long, value_parser = super::identifier)]
    pub swimlane_id: Option<String>,
    /// Requested list ID; requires --swimlane-id. Ignored by pinned Wekan.
    #[arg(long = "list", requires = "swimlane_id", value_parser = super::identifier)]
    pub list_id: Option<String>,
    /// Requested card ID; requires --list. Ignored by pinned Wekan.
    #[arg(long = "card", requires = "list_id", value_parser = super::identifier)]
    pub card_id: Option<String>,
}

pub(super) async fn execute(
    a: ListArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let response = context
        .client()
        .attachments(
            &a.board_id,
            a.swimlane_id.as_deref(),
            a.list_id.as_deref(),
            a.card_id.as_deref(),
            context.record().token(),
        )
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "attachment collection",
                "the attachment board was not found",
            )
        })?;
    let requested_scope = if a.card_id.is_some() {
        AttachmentScope::Card
    } else if a.list_id.is_some() {
        AttachmentScope::List
    } else if a.swimlane_id.is_some() {
        AttachmentScope::Swimlane
    } else {
        AttachmentScope::Board
    };
    Ok(CommandSuccess::AttachmentList(
        AttachmentCollectionSuccess {
            board_id: a.board_id,
            swimlane_id: a.swimlane_id,
            list_id: a.list_id,
            card_id: a.card_id,
            requested_scope,
            effective_scope: AttachmentScope::Board,
            count: response.count,
            attachments: response.attachments,
        },
    ))
}
