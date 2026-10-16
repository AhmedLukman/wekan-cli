mod delete;
mod download;
mod export;
mod get;
mod list;
#[cfg(test)]
mod tests;
mod transfer;
mod upload;

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, credentials::CredentialStore,
    error::AppError, input::ConfirmationProvider,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct AttachmentArgs {
    #[command(subcommand)]
    pub command: AttachmentCommand,
}

#[derive(Debug, Subcommand)]
pub enum AttachmentCommand {
    /// Export a native artifact to a file or validated raw stdout.
    Export(export::ExportArgs),
    /// List attachments. Wekan v11.06 ignores deeper selectors and returns all board attachments.
    List(list::ListArgs),
    /// Read attachment metadata and versions.
    Get(get::GetArgs),
    /// Upload a file or stdin (-). Pinned Wekan JSON uploads return HTTP 408.
    Upload(upload::UploadArgs),
    /// Download an attachment to a file or raw stdout (-).
    Download(download::DownloadArgs),
    /// Copy an attachment to a card. Pinned Wekan JSON copies return HTTP 408.
    Copy(transfer::TransferArgs),
    /// Move an attachment to a card. Pinned Wekan JSON moves return HTTP 408.
    Move(transfer::TransferArgs),
    /// Permanently delete an attachment; requires confirmation or --yes.
    Delete(delete::DeleteArgs),
    /// Upload and select a board background. Pinned Wekan JSON uploads return HTTP 408.
    UploadBackground(upload::BackgroundArgs),
    /// Download the current board background to a file or raw stdout (-).
    DownloadBackground(download::BackgroundArgs),
}

impl AttachmentCommand {
    pub(crate) fn validate_input(&self) -> Result<(), AppError> {
        match self {
            Self::Upload(a) => {
                a.source.name()?;
            }
            Self::UploadBackground(a) => {
                a.source.name()?;
            }
            _ => {}
        }
        Ok(())
    }
    pub(crate) fn download_destination(&self) -> Option<&download::DestinationArgs> {
        match self {
            Self::Download(a) => Some(&a.destination),
            Self::DownloadBackground(a) => Some(&a.destination),
            _ => None,
        }
    }
}

pub(crate) async fn dispatch(
    command: AttachmentCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        AttachmentCommand::Export(args) => export::execute(args, factory, store).await,
        AttachmentCommand::List(a) => list::execute(a, factory, store).await,
        AttachmentCommand::Get(a) => get::execute(a, factory, store).await,
        AttachmentCommand::Upload(a) => upload::execute(a, factory, store).await,
        AttachmentCommand::UploadBackground(a) => upload::background(a, factory, store).await,
        AttachmentCommand::Download(a) => {
            download::execute(Some(a.attachment_id), None, a.destination, factory, store).await
        }
        AttachmentCommand::DownloadBackground(a) => {
            download::execute(None, Some(a.board_id), a.destination, factory, store).await
        }
        AttachmentCommand::Copy(a) => transfer::execute(a, false, factory, store).await,
        AttachmentCommand::Move(a) => transfer::execute(a, true, factory, store).await,
        AttachmentCommand::Delete(a) => delete::execute(a, factory, store, confirmation).await,
    }
}

fn identifier(value: &str) -> Result<String, String> {
    let value = crate::client::trim_javascript_whitespace(value);
    if value.is_empty() || matches!(value, "." | "..") {
        Err("ID must be nonblank and cannot be . or ..".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

fn file_error(operation: &str, path: &std::path::Path, error: std::io::Error) -> AppError {
    AppError::new(
        crate::error::ErrorCode::FileIoError,
        format!(
            "could not {operation} attachment file `{}`: {}",
            crate::input::escape_terminal_text(&path.display().to_string()),
            crate::input::escape_terminal_text(&error.to_string())
        ),
        crate::exit_code::StableExitCode::Transport,
    )
}
