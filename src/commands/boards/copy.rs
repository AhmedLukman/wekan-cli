use crate::{
    client::{WekanClientFactory, boards::CopyBoardRequest},
    command_result::{BoardCopySuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
        transfer::identifier,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct CopyArgs {
    /// Source board ID. Wekan requires board administrator access.
    #[arg(value_parser = identifier)]
    pub board_id: String,
    /// Requested title, sent unchanged; Wekan chooses a unique title.
    #[arg(long)]
    pub title: Option<String>,
}
pub(super) async fn execute(
    args: CopyArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let board_id = context
        .client()
        .copy_board(
            &args.board_id,
            &CopyBoardRequest { title: args.title },
            context.record().token(),
        )
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "board copy",
                "the source board was not found",
            )
        })?;
    Ok(CommandSuccess::BoardCopied(BoardCopySuccess {
        source_board_id: args.board_id,
        board_id,
    }))
}
