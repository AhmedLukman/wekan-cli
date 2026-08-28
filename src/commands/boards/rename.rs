use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{BoardRenameSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_client_error, protocol_error_details},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    redaction::Redactor,
};

use super::{non_empty, trimmed_non_empty};

#[derive(Debug, Args)]
pub struct RenameArgs {
    /// Wekan board ID.
    #[arg(value_parser = non_empty)]
    pub board_id: String,

    /// New board title.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub title: String,
}

pub(super) async fn execute(
    args: RenameArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let renamed = context
        .client()
        .rename_board(&args.board_id, &args.title, context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "board rename", true))?;
    if renamed.board_id != args.board_id || renamed.title != args.title {
        let mut details = protocol_error_details(true);
        details.outcome_unknown = Some(true);
        return Err(AppError::new(
            ErrorCode::ProtocolError,
            "Wekan reported a different board ID or title than requested",
            StableExitCode::Transport,
        )
        .with_details(details));
    }
    Ok(CommandSuccess::BoardRenamed(BoardRenameSuccess {
        board_id: renamed.board_id,
        title: renamed.title,
    }))
}
