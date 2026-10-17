use clap::{Args, Subcommand};

use crate::commands::{
    authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
};
use crate::{
    client::WekanClientFactory,
    command_result::{BoardAttachmentCollectionSuccess, CommandSuccess},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

#[derive(Debug, Args)]
pub struct AttachmentArgs {
    #[command(subcommand)]
    pub command: AttachmentCommand,
}

#[derive(Debug, Subcommand)]
pub enum AttachmentCommand {
    /// List native legacy attachment links, including board backgrounds.
    List(ListArgs),
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = identifier)]
    pub board_id: String,
}

fn identifier(value: &str) -> Result<String, String> {
    let value = crate::client::trim_javascript_whitespace(value);
    if value.is_empty() || matches!(value, "." | "..") {
        Err("ID must be nonblank and cannot be . or ..".into())
    } else {
        Ok(value.to_owned())
    }
}

pub(super) async fn dispatch(
    command: AttachmentCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let AttachmentCommand::List(args) = command;
    let context = AuthenticatedContext::load(factory, store)?;
    let attachments = context
        .client()
        .board_attachments(&args.board_id, context.record().token())
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "board attachment list",
                "the requested Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::BoardAttachments(
        BoardAttachmentCollectionSuccess {
            board_id: args.board_id,
            count: attachments.len(),
            attachments,
        },
    ))
}
