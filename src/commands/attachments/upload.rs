use crate::{
    client::{
        WekanClientFactory,
        attachments::{MAX_FILE_BYTES, UploadAttachmentRequest, UploadBackgroundRequest},
    },
    command_result::CommandSuccess,
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    input::{BinaryInputError, acquire_binary_file},
    redaction::Redactor,
};
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct FileArgs {
    /// Local file path, or - to read binary contents from stdin.
    pub file: PathBuf,
    /// Stored filename; required for stdin, otherwise defaults to the basename.
    #[arg(long)]
    pub file_name: Option<String>,
    /// MIME type; omitted to use Wekan's default.
    #[arg(long)]
    pub file_type: Option<String>,
}

impl FileArgs {
    pub(super) fn name(&self) -> Result<String, AppError> {
        let name = match &self.file_name {
            Some(name) => name.clone(),
            None if self.file == std::path::Path::new("-") => {
                return Err(AppError::invalid_input(
                    "--file-name is required when uploading stdin (-)",
                ));
            }
            None => self
                .file
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| {
                    AppError::invalid_input(
                        "the upload needs a UTF-8 basename or an explicit --file-name",
                    )
                })?
                .to_owned(),
        };
        if name.is_empty() {
            return Err(AppError::invalid_input("--file-name must not be empty"));
        }
        Ok(name)
    }

    async fn acquire(&self) -> Result<(String, Vec<u8>), AppError> {
        let name = self.name()?;
        let data = acquire_binary_file(&self.file, MAX_FILE_BYTES)
            .await
            .map_err(|error| match error {
                BinaryInputError::Io { operation, source } => {
                    super::file_error(operation, &self.file, source)
                }
                BinaryInputError::NotRegularFile => {
                    AppError::invalid_input("upload sources must be regular files or stdin (-)")
                }
                BinaryInputError::TooLarge => AppError::invalid_input(
                    "attachment upload contents exceed the 64 MiB client file limit",
                ),
            })?;
        Ok((name, data))
    }
}

#[derive(Debug, Args)]
pub struct UploadArgs {
    #[command(flatten)]
    pub source: FileArgs,
    /// Target board ID.
    #[arg(long = "board", value_parser = super::identifier)]
    pub board_id: String,
    /// Target card's swimlane ID.
    #[arg(long, value_parser = super::identifier)]
    pub swimlane_id: String,
    /// Target card's list ID.
    #[arg(long = "list", value_parser = super::identifier)]
    pub list_id: String,
    /// Target card ID.
    #[arg(long = "card", value_parser = super::identifier)]
    pub card_id: String,
}

#[derive(Debug, Args)]
pub struct BackgroundArgs {
    #[command(flatten)]
    pub source: FileArgs,
    /// Board whose active background will be replaced.
    #[arg(long = "board", value_parser = super::identifier)]
    pub board_id: String,
}

pub(super) async fn execute(
    a: UploadArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    a.source.name()?;
    let context = AuthenticatedContext::load(factory, store)?;
    let (file_name, file_data) = a.source.acquire().await?;
    let request = UploadAttachmentRequest {
        board_id: a.board_id,
        swimlane_id: a.swimlane_id,
        list_id: a.list_id,
        card_id: a.card_id,
        file_name,
        file_data,
        file_type: a.source.file_type,
    };
    let response = context
        .client()
        .upload_attachment(&request, context.record().token())
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "attachment upload",
                "the target board or card was not found",
            )
        })?;
    Ok(CommandSuccess::AttachmentUploaded(response))
}

pub(super) async fn background(
    a: BackgroundArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    a.source.name()?;
    let context = AuthenticatedContext::load(factory, store)?;
    let (file_name, file_data) = a.source.acquire().await?;
    let request = UploadBackgroundRequest {
        board_id: a.board_id,
        file_name,
        file_data,
        file_type: a.source.file_type,
    };
    let response = context
        .client()
        .upload_background(&request, context.record().token())
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "background upload",
                "the target board was not found",
            )
        })?;
    Ok(CommandSuccess::BackgroundUploaded(response))
}
