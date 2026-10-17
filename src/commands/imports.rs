//! Shared arguments and input-error policy for native imports.
use crate::{
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{BinaryInputError, BinaryStreamInput, acquire_binary_file, acquire_binary_stream},
};
use clap::Args;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) use crate::client::imports::MAX_IMPORT_JSON_BYTES as MAX_JSON_BYTES;
pub(crate) const MAX_ZIP_BYTES: u64 = 5 * 1024 * 1024 * 1024;

#[derive(Debug, Args)]
pub struct ImportInput {
    /// Source file, or - for stdin. Format is selected by the command/source.
    pub file: PathBuf,
    /// Override the 30-second request timeout (positive integer seconds).
    #[arg(long, value_name = "SECONDS", value_parser = timeout)]
    pub timeout: Option<Duration>,
}

pub(crate) fn timeout(value: &str) -> Result<Duration, String> {
    match value.parse::<u64>() {
        Ok(seconds) if seconds > 0 => Ok(Duration::from_secs(seconds)),
        _ => Err("timeout must be a positive integer number of seconds".into()),
    }
}

fn map_input_error(error: BinaryInputError, limit_message: &str) -> AppError {
    match error {
        BinaryInputError::TooLarge => AppError::invalid_input(limit_message),
        BinaryInputError::NotRegularFile => AppError::invalid_input(
            "import inputs must be regular files or stdin (-); no request was sent",
        ),
        BinaryInputError::Io { .. } => AppError::new(
            ErrorCode::FileIoError,
            "could not read or stage the import input",
            StableExitCode::Transport,
        ),
    }
}

pub(crate) async fn acquire(path: &Path) -> Result<Vec<u8>, AppError> {
    acquire_binary_file(path, MAX_JSON_BYTES)
        .await
        .map_err(|error| {
            map_input_error(
                error,
                "import input exceeds the 50 MiB client limit; no request was sent",
            )
        })
}

pub(crate) async fn acquire_zip(path: &Path) -> Result<BinaryStreamInput, AppError> {
    acquire_binary_stream(path, MAX_ZIP_BYTES)
        .await
        .map_err(|error| {
            map_input_error(
                error,
                "ZIP input exceeds the 5 GiB client archive limit; no request was sent",
            )
        })
}
