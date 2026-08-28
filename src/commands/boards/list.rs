use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{BoardListScope, BoardListSuccess, BoardSummary, CommandSuccess},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

#[derive(Debug, Args)]
pub struct ListArgs {
    /// List all public boards instead of the authenticated user's active boards.
    #[arg(long)]
    pub public: bool,
}

pub(super) async fn execute(
    args: ListArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let (scope, boards) = if args.public {
        (
            BoardListScope::Public,
            context
                .client()
                .public_boards(context.record().token())
                .await
                .map_err(|error| map_client_error(error, &redactor, "public board list", false))?,
        )
    } else {
        (
            BoardListScope::Active,
            context
                .client()
                .user_boards(context.record().user_id(), context.record().token())
                .await
                .map_err(|error| map_client_error(error, &redactor, "active board list", false))?,
        )
    };
    Ok(CommandSuccess::BoardList(BoardListSuccess {
        scope,
        boards: boards
            .into_iter()
            .map(|board| BoardSummary {
                board_id: board.board_id,
                title: board.title,
            })
            .collect(),
    }))
}
