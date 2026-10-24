use crate::{
    client::WekanClientFactory,
    command_result::{
        AttachmentRestoreSuccess, CommandSuccess, DeletedAttachmentCollectionSuccess,
    },
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct DeletedArgs {
    /// Wekan board ID.
    #[arg(long="board",value_parser=super::identifier)]
    board_id: String,
}
#[derive(Debug, Args)]
pub struct RestoreArgs {
    /// Wekan attachment ID.
    #[arg(value_parser=super::identifier)]
    attachment_id: String,
    /// Wekan board ID.
    #[arg(long="board",value_parser=super::identifier)]
    board_id: String,
}
pub(super) async fn list(
    args: DeletedArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let attachments = context
        .client()
        .deleted_board_attachments(&args.board_id, context.record().token())
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &redactor,
                "deleted attachment list",
                "the requested Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::DeletedAttachments(
        DeletedAttachmentCollectionSuccess {
            board_id: args.board_id,
            count: attachments.len(),
            attachments,
        },
    ))
}
pub(super) async fn restore(
    args: RestoreArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let response = context
        .client()
        .restore_board_attachment(
            &args.board_id,
            &args.attachment_id,
            context.record().token(),
        )
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &redactor,
                "attachment restore",
                "the requested Wekan attachment was not found on that board",
            )
        })?;
    Ok(CommandSuccess::AttachmentRestored(
        AttachmentRestoreSuccess {
            board_id: args.board_id,
            attachment_id: args.attachment_id,
            restored: response.restored,
        },
    ))
}
