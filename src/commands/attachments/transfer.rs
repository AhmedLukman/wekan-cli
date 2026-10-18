use crate::{
    client::{WekanClientFactory, attachments::TransferAttachmentRequest},
    command_result::CommandSuccess,
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct TransferArgs {
    /// Source attachment ID.
    #[arg(value_parser = super::identifier)]
    pub attachment_id: String,
    /// Destination board ID.
    #[arg(long = "target-board", value_parser = super::identifier)]
    pub target_board_id: String,
    /// Destination card's swimlane ID.
    #[arg(long, value_parser = super::identifier)]
    pub target_swimlane_id: String,
    /// Destination card's list ID.
    #[arg(long = "target-list", value_parser = super::identifier)]
    pub target_list_id: String,
    /// Destination card ID.
    #[arg(long = "target-card", value_parser = super::identifier)]
    pub target_card_id: String,
}

pub(super) async fn execute(
    a: TransferArgs,
    moving: bool,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let request = TransferAttachmentRequest {
        attachment_id: a.attachment_id,
        target_board_id: a.target_board_id,
        target_swimlane_id: a.target_swimlane_id,
        target_list_id: a.target_list_id,
        target_card_id: a.target_card_id,
    };
    let error = |e| {
        map_client_error_with_not_found(
            e,
            &Redactor::with_secret(context.record().token()),
            "attachment transfer",
            "the source attachment or target card was not found",
        )
    };
    if moving {
        Ok(CommandSuccess::AttachmentMoved(
            context
                .client()
                .move_attachment(&request, context.record().token())
                .await
                .map_err(error)?,
        ))
    } else {
        Ok(CommandSuccess::AttachmentCopied(
            context
                .client()
                .copy_attachment(&request, context.record().token())
                .await
                .map_err(error)?,
        ))
    }
}
