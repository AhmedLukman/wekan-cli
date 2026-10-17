use crate::{
    client::{
        WekanClientFactory,
        boards::import::{BoardImportRequest, ImportSource},
        imports::PreparedImportRequest,
    },
    command_result::{BoardImportSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_mutation_client_error_with_not_found,
        imports::{self, ImportInput},
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Args)]
pub struct ImportArgs {
    #[command(flatten)]
    pub input: ImportInput,
    /// Native source engine. Omit for /api/boards/import; explicit wekan uses /api/boards/import/wekan.
    #[arg(long, value_parser = ImportSource::ALL.map(ImportSource::as_str))]
    pub source: Option<String>,
    /// JSON object mapping source member identifiers to local Wekan user IDs.
    #[arg(long, value_name = "JSON", conflicts_with = "members_mapping_file")]
    pub members_mapping: Option<String>,
    /// Read member mapping from a regular UTF-8 JSON file (50 MiB limit).
    #[arg(long, value_name = "PATH")]
    pub members_mapping_file: Option<PathBuf>,
    /// CSV delimiter (one ASCII character, or backslash-t for tab); comma by default. CSV only.
    #[arg(long, value_name = "CHAR", value_parser = delimiter)]
    pub delimiter: Option<u8>,
}

fn delimiter(value: &str) -> Result<u8, String> {
    if value == "\\t" {
        return Ok(b'\t');
    }
    match value.as_bytes() {
        [byte] if byte.is_ascii() && !matches!(*byte, 0 | b'\r' | b'\n' | b'"') => Ok(*byte),
        _ => Err(
            "delimiter must be one ASCII character other than NUL, CR, LF or quote, or \\t".into(),
        ),
    }
}

impl ImportArgs {
    pub(crate) fn validate(&self) -> Result<(), AppError> {
        if self.delimiter.is_some() && self.source.as_deref() != Some("csv") {
            return Err(AppError::invalid_input("--delimiter requires --source csv"));
        }
        if self.members_mapping_file.as_deref() == Some(Path::new("-")) {
            return Err(AppError::invalid_input(
                "--members-mapping-file requires a regular file; stdin is reserved for the export document",
            ));
        }
        Ok(())
    }
}

fn json_input(bytes: &[u8], description: &str) -> Result<Value, AppError> {
    serde_json::from_slice(bytes).map_err(|error| {
        AppError::invalid_input(format!(
            "expected UTF-8 JSON {description} at line {}, column {}; no request was sent",
            error.line(),
            error.column()
        ))
    })
}

pub(crate) async fn execute(
    args: ImportArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    args.validate()?;
    let source = args
        .source
        .as_deref()
        .map(str::parse::<ImportSource>)
        .transpose()
        .expect("parser validated the import source");
    let mapping = if let Some(text) = args.members_mapping {
        if text.len() > imports::MAX_JSON_BYTES {
            return Err(AppError::invalid_input(
                "member mapping exceeds the 50 MiB client limit",
            ));
        }
        Some(text.into_bytes())
    } else if let Some(path) = args.members_mapping_file {
        Some(imports::acquire(&path).await?)
    } else {
        None
    };
    let members_mapping = mapping
        .map(|bytes| {
            serde_json::from_slice::<BTreeMap<String, String>>(&bytes).map_err(|_| {
                AppError::invalid_input(
                    "member mapping must be a JSON object with string values; no request was sent",
                )
            })
        })
        .transpose()?;
    let bytes = imports::acquire(&args.input.file).await?;
    let request = match source {
        Some(ImportSource::Csv) => {
            let rows = csv::ReaderBuilder::new()
                .has_headers(false)
                .flexible(true)
                .delimiter(args.delimiter.unwrap_or(b','))
                .from_reader(bytes.as_slice())
                .records()
                .map(|row| row.map(|r| r.iter().map(str::to_owned).collect::<Vec<_>>()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| {
                    AppError::invalid_input(
                        "could not decode UTF-8 CSV records; no request was sent",
                    )
                })?;
            BoardImportRequest {
                board: serde_json::to_value(rows).expect("CSV strings are serializable"),
                members_mapping,
            }
        }
        Some(ImportSource::Excel) => BoardImportRequest::excel(&bytes, members_mapping),
        _ => {
            let value = json_input(&bytes, "export document")?;
            if !value.is_object() && !value.is_array() {
                return Err(AppError::invalid_input(
                    "export document must be a JSON object or array; no request was sent",
                ));
            }
            BoardImportRequest {
                board: value,
                members_mapping,
            }
        }
    };
    let body = PreparedImportRequest::new(&request)
        .map_err(|error| AppError::invalid_input(error.to_string()))?;
    let context = AuthenticatedContext::load(factory, store)?;
    let response = context
        .client()
        .import_board(source, body, context.record().token(), args.input.timeout)
        .await
        .map_err(|error| {
            map_mutation_client_error_with_not_found(
                error,
                &Redactor::with_secret(context.record().token()),
                "board import",
                "the import target was not found",
            )
        })?;
    Ok(CommandSuccess::BoardImported(BoardImportSuccess {
        board_id: response.board_id,
        source: source.map_or("native", ImportSource::as_str).into(),
        mutation_confirmed: false,
    }))
}
