use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{
        CancellationSuccess, ChecklistDeleteMode, ChecklistDeleteSuccess, CommandSuccess,
        DestructiveOperation,
    },
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_client_error_with_not_found,
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
#[command(
    after_help = "Example:\n  wekan checklist delete checklist-id --board board-id --card card-id --yes"
)]
pub struct DeleteArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan card ID.
    #[arg(long = "card", value_parser = non_empty, help_heading = "Target")]
    pub card_id: String,

    /// Wekan checklist ID to permanently delete.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub checklist_id: String,

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
            "the stored credential changed while preparing checklist deletion; no HTTP request was sent",
            StableExitCode::Credential,
        )
        .with_credential_stored(true));
    }
    let request = ConfirmationRequest::new(
        DestructiveOperation::ChecklistDelete,
        format!(
            "Permanently delete Wekan checklist `{}` from card `{}` on board `{}`?",
            escape_terminal_text(&args.checklist_id),
            escape_terminal_text(&args.card_id),
            escape_terminal_text(&args.board_id)
        ),
    );
    if confirm_or_skip(&args.confirmation, confirmation, &request)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::ChecklistDelete,
        )));
    }

    let redactor = Redactor::with_secret(context.record().token());
    let deletion = context
        .client()
        .delete_checklist(
            &args.board_id,
            &args.card_id,
            &args.checklist_id,
            context.record().token(),
        )
        .await;
    drop(credential_mutation);
    let deleted = deletion.map_err(|error| {
        map_client_error_with_not_found(
            error,
            &redactor,
            "checklist deletion",
            "the requested Wekan board or checklist was not found",
        )
    })?;
    Ok(CommandSuccess::ChecklistDeleted(ChecklistDeleteSuccess {
        board_id: args.board_id,
        card_id: args.card_id,
        checklist_id: deleted.checklist_id,
        deleted: true,
        delete_mode: ChecklistDeleteMode::Hard,
    }))
}
