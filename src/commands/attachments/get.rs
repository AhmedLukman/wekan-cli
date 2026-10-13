use crate::{
    client::WekanClientFactory,
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
pub struct GetArgs {
    /// Wekan attachment ID.
    #[arg(value_parser = super::identifier)]
    pub attachment_id: String,
}

pub(super) async fn execute(
    a: GetArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let response = context
        .client()
        .attachment_info(&a.attachment_id, context.record().token())
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "attachment metadata",
                "the attachment was not found",
            )
        })?;
    Ok(CommandSuccess::AttachmentShown(response))
}
