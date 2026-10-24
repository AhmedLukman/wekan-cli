use super::non_empty;
use crate::{
    client::{CreateCommentRequest, WekanClientFactory, comments::is_blank_comment_text},
    command_result::{CommandSuccess, CommentCreateSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Wekan comment ID.
    #[arg(value_parser = non_empty)]
    comment_id: String,
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    board_id: String,
    /// Wekan card ID.
    #[arg(long = "card", value_parser = non_empty)]
    card_id: String,
    /// Replacement comment text; Wekan trims JavaScript whitespace.
    #[arg(long, value_parser = comment_text)]
    text: String,
}
fn comment_text(value: &str) -> Result<String, String> {
    if is_blank_comment_text(value) {
        Err("comment text must not be empty or JavaScript whitespace".to_owned())
    } else {
        Ok(value.to_owned())
    }
}
pub(super) async fn execute(
    args: UpdateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let result = context
        .client()
        .update_comment(
            &args.board_id,
            &args.card_id,
            &args.comment_id,
            &CreateCommentRequest { text: args.text },
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "comment update",
                "the requested Wekan comment was not found on that card",
            )
        })?;
    Ok(CommandSuccess::CommentUpdated(CommentCreateSuccess {
        board_id: args.board_id,
        card_id: args.card_id,
        comment_id: result.comment_id,
    }))
}
