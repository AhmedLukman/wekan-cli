use crate::{
    client::admin::{MAX_SETTINGS_BYTES, Object},
    credentials::{SecretInputProvider, SecretSource},
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{BinaryInputError, PayloadSource},
};
use clap::{ArgGroup, Args};
use secrecy::SecretString;
use serde::de::DeserializeOwned;
use std::path::PathBuf;

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("settings_source").args(["settings_file", "settings_stdin"]).multiple(false)))]
pub(super) struct SourceArgs {
    /// Read a strict camelCase settings object from a regular UTF-8 file (50 MiB).
    #[arg(long, value_name = "PATH")]
    settings_file: Option<PathBuf>,
    /// Read a strict camelCase settings object from UTF-8 stdin (50 MiB).
    #[arg(long)]
    settings_stdin: bool,
}
impl SourceArgs {
    pub(super) async fn read<T: DeserializeOwned>(self) -> Result<Option<T>, AppError> {
        let source = if let Some(path) = self.settings_file {
            PayloadSource::File(path)
        } else if self.settings_stdin {
            PayloadSource::Stdin
        } else {
            return Ok(None);
        };
        let bytes = source
            .acquire_bounded(MAX_SETTINGS_BYTES)
            .await
            .map_err(input_error)?;
        serde_json::from_slice::<Object<T>>(&bytes).map(|v| Some(v.0)).map_err(|e| AppError::invalid_input(format!("expected a typed settings JSON object with supported camelCase properties at line {}, column {}; no request was sent", e.line(), e.column())))
    }
}
fn input_error(error: BinaryInputError) -> AppError {
    match error {
        BinaryInputError::TooLarge => {
            AppError::invalid_input("settings input exceeds the 50 MiB limit; no request was sent")
        }
        BinaryInputError::NotRegularFile => AppError::invalid_input(
            "settings file input must be a regular file; no request was sent",
        ),
        BinaryInputError::Io { .. } => AppError::new(
            ErrorCode::FileIoError,
            "could not read settings input; no request was sent",
            StableExitCode::Transport,
        ),
    }
}
pub(super) async fn secret(
    file: Option<PathBuf>,
    stdin: bool,
    secrets: &dyn SecretInputProvider,
) -> Result<Option<SecretString>, AppError> {
    let source = if let Some(path) = file {
        SecretSource::File(path)
    } else if stdin {
        SecretSource::Stdin
    } else {
        return Ok(None);
    };
    secrets
        .read_storage_secret(source, MAX_SETTINGS_BYTES)
        .await
        .map(Some)
}
pub(super) fn validate_fields<T>(fields: &[T]) -> Result<(), AppError> {
    if fields.is_empty() {
        return Err(AppError::invalid_input(
            "supply at least one writable setting; no request was sent",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn settings_source_size_is_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("large.json");
        std::fs::File::create(&file)
            .unwrap()
            .set_len(MAX_SETTINGS_BYTES as u64 + 1)
            .unwrap();
        let error = SourceArgs {
            settings_file: Some(file),
            settings_stdin: false,
        }
        .read::<crate::client::admin::settings::UpdateGlobalSettings>()
        .await
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidInput);
        assert!(error.message().contains("50 MiB"));
    }
}
