use crate::{
    client::WekanClientFactory,
    command_result::{CancellationSuccess, CommandSuccess, DestructiveOperation},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_client_error_with_not_found,
        credential_ops::{lock_credential_mutation, map_credential_load_error},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip, escape_terminal_text,
    },
    redaction::Redactor,
};
use clap::Args;
use time::OffsetDateTime;

#[derive(Debug, Args)]
pub struct DeleteArgs {
    /// Wekan attachment ID to soft-delete.
    #[arg(value_parser = super::identifier)]
    pub attachment_id: String,
    /// Use the board-scoped delete endpoint; omit to use the legacy attachment endpoint.
    #[arg(long="board", value_parser=super::identifier)]
    pub board_id: Option<String>,
    #[command(flatten)]
    pub confirmation: ConfirmationArgs,
}

pub(super) async fn execute(
    a: DeleteArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let guard = lock_credential_mutation(store, context.credential_target())?;
    let request = ConfirmationRequest::new(
        DestructiveOperation::AttachmentDelete,
        format!(
            "Delete Wekan attachment `{}`? Its file is retained for restoration.",
            escape_terminal_text(&a.attachment_id)
        ),
    );
    if confirm_or_skip(&a.confirmation, confirmation, &request)? == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::AttachmentDelete,
        )));
    }
    let record = guard
        .load()
        .map_err(map_credential_load_error)?
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::CredentialNotFound,
                "the credential used to prepare attachment deletion is no longer stored; no HTTP request was sent",
                StableExitCode::Credential,
            )
        })?;
    if !record.matches(context.record()) {
        return Err(AppError::new(ErrorCode::CredentialStoreFailed, "the stored credential changed while preparing attachment deletion; no HTTP request was sent", StableExitCode::Credential).with_credential_stored(true));
    }
    if record.token_expires() <= OffsetDateTime::now_utc() {
        return Err(AppError::new(
            ErrorCode::CredentialExpired,
            format!(
                "the stored credential expired at {}; no HTTP request was sent",
                context.token_expires()
            ),
            StableExitCode::Server,
        )
        .with_details(ErrorDetails {
            token_expires: Some(context.token_expires().to_owned()),
            ..ErrorDetails::default()
        }));
    }
    let response = if let Some(board_id) = a.board_id {
        context
            .client()
            .soft_delete_board_attachment(&board_id, &a.attachment_id, context.record().token())
            .await
            .map(|response| {
                CommandSuccess::AttachmentSoftDeleted(
                    crate::command_result::AttachmentSoftDeleteSuccess {
                        board_id,
                        attachment_id: a.attachment_id,
                        deleted: response.deleted,
                        batch_id: response.batch_id,
                    },
                )
            })
    } else {
        context
            .client()
            .delete_attachment(&a.attachment_id, context.record().token())
            .await
            .map(CommandSuccess::AttachmentDeleted)
    };
    drop(guard);
    let response = response.map_err(|e| {
        map_client_error_with_not_found(
            e,
            &Redactor::with_secret(context.record().token()),
            "attachment deletion",
            "the attachment was not found",
        )
    })?;
    Ok(response)
}
