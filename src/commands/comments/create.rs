use clap::Args;

use crate::{
    client::{CreateCommentRequest, WekanClientFactory},
    command_result::{CommandSuccess, CommentCreateSuccess},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
#[command(
    after_help = "Example:\n  wekan comment create --board board-id --card card-id --text \"Ready for review\""
)]
#[command(next_help_heading = "Fields")]
pub struct CreateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan card ID.
    #[arg(long = "card", value_parser = non_empty, help_heading = "Target")]
    pub card_id: String,

    /// Comment text.
    #[arg(long, value_parser = non_empty)]
    pub text: String,
}

pub(super) async fn execute(
    args: CreateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let created = context
        .client()
        .create_comment(
            &args.board_id,
            &args.card_id,
            &CreateCommentRequest { text: args.text },
            context.record().token(),
        )
        .await
        .map_err(|error| map_client_error(error, &redactor, "comment creation", true))?;
    Ok(CommandSuccess::CommentCreated(CommentCreateSuccess {
        board_id: args.board_id,
        card_id: args.card_id,
        comment_id: created.comment_id,
    }))
}
