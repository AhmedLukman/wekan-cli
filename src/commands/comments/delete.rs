use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{
        CancellationSuccess, CommandSuccess, CommentDeleteMode, CommentDeleteSuccess,
        DestructiveOperation,
    },
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_mutation_client_error_with_not_found, protocol_error_details},
        credential_ops::{lock_credential_mutation, map_credential_load_error},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip, escape_terminal_text,
    },
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    pub board_id: String,

    /// Wekan card ID.
    #[arg(long = "card", value_parser = non_empty)]
    pub card_id: String,

    /// Wekan comment ID to permanently delete.
    #[arg(value_parser = non_empty)]
    pub comment_id: String,

    #[command(flatten)]
    pub confirmation: ConfirmationArgs,
}

pub(super) async fn execute(
    args: DeleteArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let credential_mutation =
        lock_credential_mutation(credential_store, context.credential_target())?;
    let guarded_record = credential_mutation
        .load()
        .map_err(map_credential_load_error)?
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::CredentialNotFound,
                "the credential used to prepare this request is no longer stored",
                StableExitCode::Credential,
            )
        })?;
    if !guarded_record.matches(context.record()) {
        return Err(AppError::new(
            ErrorCode::CredentialStoreFailed,
            "the stored credential changed while preparing comment deletion; no HTTP request was sent",
            StableExitCode::Credential,
        )
        .with_credential_stored(true));
    }
    let request = ConfirmationRequest::new(
        DestructiveOperation::CommentDelete,
        format!(
            "Permanently delete Wekan comment `{}` from card `{}` on board `{}`?",
            escape_terminal_text(&args.comment_id),
            escape_terminal_text(&args.card_id),
            escape_terminal_text(&args.board_id)
        ),
    );
    if confirm_or_skip(&args.confirmation, confirmation, &request)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::CommentDelete,
        )));
    }

    let redactor = Redactor::with_secret(context.record().token());
    let deletion = context
        .client()
        .delete_comment(
            &args.board_id,
            &args.card_id,
            &args.comment_id,
            context.record().token(),
        )
        .await;
    drop(credential_mutation);
    let deleted = deletion.map_err(|error| {
        map_mutation_client_error_with_not_found(
            error,
            &redactor,
            "comment deletion",
            "the requested Wekan board or comment was not found",
        )
    })?;
    if deleted.card_id != args.card_id {
        let mut details = protocol_error_details(true);
        details.outcome_unknown = Some(true);
        return Err(AppError::new(
            ErrorCode::ProtocolError,
            "Wekan reported a different card ID after comment deletion than requested",
            StableExitCode::Transport,
        )
        .with_details(details));
    }
    Ok(CommandSuccess::CommentDeleted(CommentDeleteSuccess {
        board_id: args.board_id,
        card_id: deleted.card_id,
        comment_id: args.comment_id,
        deleted: true,
        delete_mode: CommentDeleteMode::Hard,
    }))
}
