use clap::Args;

use crate::{
    client::{CommentDocument, WekanClientFactory},
    command_result::{CommandSuccess, CommentDetail},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
pub struct GetArgs {
    /// Wekan board ID.
    #[arg(value_parser = non_empty)]
    pub board_id: String,

    /// Wekan card ID.
    #[arg(value_parser = non_empty)]
    pub card_id: String,

    /// Wekan comment ID.
    #[arg(value_parser = non_empty)]
    pub comment_id: String,
}

pub(super) async fn execute(
    args: GetArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let comment = context
        .client()
        .comment(
            &args.board_id,
            &args.card_id,
            &args.comment_id,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "comment lookup",
                "the requested Wekan comment was not found on that card",
            )
        })?;
    Ok(CommandSuccess::CommentShown(comment_detail(comment)))
}

fn comment_detail(comment: CommentDocument) -> CommentDetail {
    CommentDetail {
        comment_id: comment.comment_id,
        board_id: comment.board_id,
        card_id: comment.card_id,
        text: comment.text,
        parent_id: comment.parent_id,
        created_at: comment.created_at,
        modified_at: comment.modified_at,
        author_id: comment.author_id,
    }
}
