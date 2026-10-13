use crate::{
    client::{WekanClientFactory, attachments::decode_file},
    command_result::{AttachmentDownloadSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    output::OutputFormat,
    redaction::Redactor,
};
use clap::Args;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Args)]
pub struct DestinationArgs {
    /// Destination file path, or - for stdout (requires --output raw).
    #[arg(long)]
    pub destination: PathBuf,
    /// Replace an existing destination file after validating the complete response.
    #[arg(long)]
    pub overwrite: bool,
}

impl DestinationArgs {
    pub(crate) fn is_stdout(&self) -> bool {
        self.destination == Path::new("-")
    }
    pub(crate) fn validate_output(&self, format: OutputFormat) -> Result<(), AppError> {
        if self.is_stdout() != (format == OutputFormat::Raw) {
            return Err(AppError::invalid_input(
                "--destination - requires --output raw; file destinations require human or json output",
            ));
        }
        self.validate()
    }
    fn validate(&self) -> Result<(), AppError> {
        if self.is_stdout() && self.overwrite {
            return Err(AppError::invalid_input(
                "--overwrite cannot be used with --destination -",
            ));
        }
        if self.destination.as_os_str().is_empty() {
            return Err(AppError::invalid_input("--destination must not be empty"));
        }
        Ok(())
    }
}

#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// Wekan attachment ID.
    #[arg(value_parser = super::identifier)]
    pub attachment_id: String,
    #[command(flatten)]
    pub destination: DestinationArgs,
}

#[derive(Debug, Args)]
pub struct BackgroundArgs {
    /// Board whose current background will be downloaded.
    #[arg(long = "board", value_parser = super::identifier)]
    pub board_id: String,
    #[command(flatten)]
    pub destination: DestinationArgs,
}

pub(super) async fn execute(
    id: Option<String>,
    board: Option<String>,
    destination: DestinationArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    destination.validate()?;
    let context = AuthenticatedContext::load(factory, store)?;
    let map_error = |e| {
        map_client_error_with_not_found(
            e,
            &Redactor::with_secret(context.record().token()),
            "attachment download",
            "the attachment, board background or stored file was not found",
        )
    };
    let (mut result, bytes) = if let Some(id) = id {
        let response = context
            .client()
            .download_attachment(&id, context.record().token())
            .await
            .map_err(map_error)?;
        let bytes = decode_file(&response.base64_data).map_err(map_error)?;
        (
            AttachmentDownloadSuccess {
                attachment_id: response.attachment_id,
                file_name: response.file_name,
                file_size: response.file_size,
                file_type: response.file_type,
                storage_backend: response.storage_backend,
                background_image_url: None,
                destination: destination.destination.display().to_string(),
                bytes_written: bytes.len() as u64,
                file_saved: false,
                bytes: None,
            },
            bytes,
        )
    } else {
        let response = context
            .client()
            .download_background(
                board
                    .as_deref()
                    .expect("background download requires a board"),
                context.record().token(),
            )
            .await
            .map_err(map_error)?;
        let bytes = decode_file(&response.base64_data).map_err(map_error)?;
        (
            AttachmentDownloadSuccess {
                attachment_id: response.attachment_id,
                file_name: response.file_name,
                file_size: response.file_size,
                file_type: response.file_type,
                storage_backend: response.storage_backend,
                background_image_url: Some(response.background_image_url),
                destination: destination.destination.display().to_string(),
                bytes_written: bytes.len() as u64,
                file_saved: false,
                bytes: None,
            },
            bytes,
        )
    };
    if destination.is_stdout() {
        result.bytes = Some(bytes);
    } else {
        save_file(&destination.destination, destination.overwrite, &bytes)?;
        result.file_saved = true;
    }
    Ok(CommandSuccess::AttachmentDownloaded(result))
}

fn save_file(path: &Path, overwrite: bool, bytes: &[u8]) -> Result<(), AppError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| super::file_error("stage", path, e))?;
    temporary
        .write_all(bytes)
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|e| super::file_error("write", path, e))?;
    let persisted = if overwrite {
        temporary.persist(path)
    } else {
        temporary.persist_noclobber(path)
    };
    persisted.map_err(|e| {
        super::file_error(
            "save (use --overwrite to replace an existing destination)",
            path,
            e.error,
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staging_preserves_existing_files_and_cleans_up_after_failure() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("download");
        save_file(&path, false, b"old").unwrap();
        assert!(save_file(&path, false, b"new").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"old");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        save_file(&path, true, &[0, 255, 0]).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), [0, 255, 0]);
        assert!(
            save_file(
                &directory.path().join("missing").join("download"),
                true,
                b"data"
            )
            .is_err()
        );
        assert!(save_file(directory.path(), true, b"data").is_err());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
