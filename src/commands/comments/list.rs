use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{CommandSuccess, CommentCollectionSuccess, CommentSummary},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
#[command(after_help = "Example:\n  wekan comment list --board board-id --card card-id")]
pub struct ListArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan card ID.
    #[arg(long = "card", value_parser = non_empty, help_heading = "Target")]
    pub card_id: String,
}

pub(super) async fn execute(
    args: ListArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let comments = context
        .client()
        .card_comments(&args.board_id, &args.card_id, context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "card comment collection", false))?;
    Ok(CommandSuccess::CommentCollection(
        CommentCollectionSuccess {
            board_id: args.board_id,
            card_id: args.card_id,
            comments: comments
                .into_iter()
                .map(|comment| CommentSummary {
                    comment_id: comment.comment_id,
                    text: comment.text,
                    author_id: comment.author_id,
                })
                .collect(),
        },
    ))
}
