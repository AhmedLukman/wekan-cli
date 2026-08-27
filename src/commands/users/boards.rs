use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{CommandSuccess, UserBoardSummary, UserBoardsSuccess},
    commands::authenticated::AuthenticatedContext,
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::{map_client_error, non_empty};

#[derive(Debug, Args)]
pub struct BoardsArgs {
    /// Wekan user ID.
    #[arg(value_parser = non_empty)]
    pub user_id: String,
}

pub(super) async fn execute(
    args: BoardsArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let boards = context
        .client()
        .user_boards(&args.user_id, context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "user board list", false))?
        .into_iter()
        .map(|board| UserBoardSummary {
            board_id: board.board_id,
            title: board.title,
        })
        .collect();
    Ok(CommandSuccess::UserBoards(UserBoardsSuccess {
        user_id: args.user_id,
        boards,
    }))
}
