use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{
        CancellationSuccess, CommandSuccess, DestructiveOperation, SwimlaneDeleteMode,
        SwimlaneDeleteSuccess,
    },
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_client_error, protocol_error_details},
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
#[command(after_help = "Example:\n  wekan swimlane delete swimlane-id --board board-id --yes")]
pub struct DeleteArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan swimlane ID to permanently delete.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub swimlane_id: String,

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
            "the stored credential changed while preparing swimlane deletion; no HTTP request was sent",
            StableExitCode::Credential,
        )
        .with_credential_stored(true));
    }
    let request = ConfirmationRequest::new(
        DestructiveOperation::SwimlaneDelete,
        format!(
            "Permanently delete Wekan swimlane `{}` on board `{}`? Wekan may also delete associated lists or cards.",
            escape_terminal_text(&args.swimlane_id),
            escape_terminal_text(&args.board_id)
        ),
    );
    if confirm_or_skip(&args.confirmation, confirmation, &request)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::SwimlaneDelete,
        )));
    }

    let redactor = Redactor::with_secret(context.record().token());
    let deletion = context
        .client()
        .delete_swimlane(&args.board_id, &args.swimlane_id, context.record().token())
        .await;
    drop(credential_mutation);
    let deleted =
        deletion.map_err(|error| map_client_error(error, &redactor, "swimlane deletion", true))?;
    if deleted.swimlane_id != args.swimlane_id {
        let mut details = protocol_error_details(true);
        details.outcome_unknown = Some(true);
        return Err(AppError::new(
            ErrorCode::ProtocolError,
            "Wekan reported a different deleted swimlane ID than requested",
            StableExitCode::Transport,
        )
        .with_details(details));
    }
    Ok(CommandSuccess::SwimlaneDeleted(SwimlaneDeleteSuccess {
        board_id: args.board_id,
        swimlane_id: deleted.swimlane_id,
        deleted: true,
        delete_mode: SwimlaneDeleteMode::Hard,
    }))
}
