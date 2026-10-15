use crate::{
    client::rules::Object,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{BinaryInputError, PayloadSource},
};
use clap::{ArgGroup, Args};
use serde::de::DeserializeOwned;
use std::path::PathBuf;

const MAX_JSON_BYTES: usize = 50 * 1024 * 1024;

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("rule_source").args(["rule", "rule_file", "rule_stdin"]).required(true).multiple(false)))]
pub(super) struct DefinitionArgs {
    /// Typed camelCase rule JSON object (50 MiB limit).
    #[arg(long, value_name = "JSON")]
    rule: Option<String>,
    /// Read a typed rule JSON object from a regular UTF-8 file (50 MiB limit).
    #[arg(long, value_name = "PATH")]
    rule_file: Option<PathBuf>,
    /// Read a typed rule JSON object from UTF-8 standard input (50 MiB limit).
    #[arg(long)]
    rule_stdin: bool,
}
impl DefinitionArgs {
    pub(super) async fn read<T: DeserializeOwned>(self) -> Result<T, AppError> {
        let source = if let Some(text) = self.rule {
            PayloadSource::Text(text)
        } else if let Some(path) = self.rule_file {
            PayloadSource::File(path)
        } else {
            PayloadSource::Stdin
        };
        let bytes = source
            .acquire_bounded(MAX_JSON_BYTES)
            .await
            .map_err(|error| match error {
                BinaryInputError::TooLarge => AppError::invalid_input(
                    "rule JSON input exceeds the 50 MiB client limit; no request was sent",
                ),
                BinaryInputError::NotRegularFile => AppError::invalid_input(
                    "rule JSON file input must be a regular file; no request was sent",
                ),
                BinaryInputError::Io { .. } => AppError::new(
                    ErrorCode::FileIoError,
                    "could not read the rule JSON input; no request was sent",
                    StableExitCode::Transport,
                ),
            })?;
        serde_json::from_slice::<Object<T>>(&bytes).map(|v| v.0).map_err(|error| AppError::invalid_input(format!(
            "expected a typed rule JSON object with supported camelCase properties at line {}, column {}; no request was sent",
            error.line(), error.column())))
    }
}
