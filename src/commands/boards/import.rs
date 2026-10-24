use crate::{
    client::{
        WekanClientFactory,
        boards::import::{
            BoardImportOptions, BoardImportRequest, CsvImportMapping, ImportAttachmentAuth,
            ImportSource,
        },
        imports::PreparedImportRequest,
    },
    command_result::{BoardImportSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_client_error_with_not_found,
        imports::{self, ImportInput},
    },
    credentials::{CredentialStore, SecretInputProvider, SecretSource},
    error::AppError,
    redaction::Redactor,
};
use clap::Args;
use secrecy::ExposeSecret;
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
    /// Split generalized imports into one board per source project/swimlane.
    #[arg(long, value_parser = ["swimlane"])]
    pub split_by: Option<String>,
    /// Native source-member handling; requires --source (use wekan for a Wekan export).
    #[arg(long, value_parser = ["map", "placeholder", "me"])]
    pub members_mode: Option<String>,
    /// CSV/Excel column mapping as a JSON object.
    #[arg(long, value_name = "JSON")]
    pub csv_mapping: Option<String>,
    /// JSON file containing optional url, username and token for attachment downloads.
    #[arg(long, value_name = "PATH")]
    pub attachment_auth_file: Option<PathBuf>,
    /// File encoding for Plane, Notion or Vikunja; auto uses the filename extension.
    #[arg(long, default_value = "auto", value_parser = ["auto", "json", "text", "xlsx", "zip"])]
    pub input_format: String,
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
        if self.members_mode.is_some() && self.source.is_none() {
            return Err(AppError::invalid_input(
                "--members-mode requires --source; use --source wekan for a Wekan export",
            ));
        }
        if self.delimiter.is_some() && self.source.as_deref() != Some("csv") {
            return Err(AppError::invalid_input("--delimiter requires --source csv"));
        }
        if self.members_mapping_file.as_deref() == Some(Path::new("-")) {
            return Err(AppError::invalid_input(
                "--members-mapping-file requires a regular file; stdin is reserved for the export document",
            ));
        }
        if self.attachment_auth_file.as_deref() == Some(Path::new("-")) {
            return Err(AppError::invalid_input(
                "--attachment-auth-file requires a regular file; stdin is reserved for the export document",
            ));
        }
        if self.csv_mapping.is_some() && !matches!(self.source.as_deref(), Some("csv" | "excel")) {
            return Err(AppError::invalid_input(
                "--csv-mapping requires --source csv or excel",
            ));
        }
        if self.input_format != "auto"
            && !matches!(self.source.as_deref(), Some("plane" | "notion" | "vikunja"))
        {
            return Err(AppError::invalid_input(
                "--input-format requires --source plane, notion or vikunja",
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
    secrets: &dyn SecretInputProvider,
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
    let attachment_auth: Option<ImportAttachmentAuth> = if let Some(path) =
        args.attachment_auth_file
    {
        let input = secrets
            .read_storage_secret(SecretSource::File(path), imports::MAX_JSON_BYTES)
            .await?;
        Some(serde_json::from_str(input.expose_secret()).map_err(|_| AppError::invalid_input("attachment authentication must contain only optional string fields url, username and token; no request was sent"))?)
    } else {
        None
    };
    let csv_mapping: Option<CsvImportMapping> = args
        .csv_mapping
        .as_deref()
        .map(|text| {
            serde_json::from_str(text).map_err(|_| {
                AppError::invalid_input("invalid CSV mapping object; no request was sent")
            })
        })
        .transpose()?;
    let bytes = imports::acquire(&args.input.file).await?;
    let extension = args
        .input
        .file
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let encoding = if args.input_format == "auto" {
        extension.as_str()
    } else {
        &args.input_format
    };
    let mut request = match source {
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
                options: Default::default(),
                board: serde_json::to_value(rows).expect("CSV strings are serializable"),
                members_mapping,
            }
        }
        Some(source) if source.is_workbook() => BoardImportRequest::excel(&bytes, members_mapping),
        Some(ImportSource::Plane) if encoding == "xlsx" => {
            BoardImportRequest::xlsx(&bytes, members_mapping)
        }
        Some(ImportSource::Plane | ImportSource::Notion | ImportSource::Vikunja)
            if encoding == "zip" =>
        {
            BoardImportRequest::zip(&bytes, members_mapping)
        }
        Some(source) if source.is_text() => BoardImportRequest {
            options: Default::default(),
            members_mapping,
            board: Value::String(
                String::from_utf8(bytes)
                    .map_err(|_| {
                        AppError::invalid_input(
                            "the source document must be UTF-8 text; no request was sent",
                        )
                    })?
                    .trim_start_matches('\u{feff}')
                    .to_owned(),
            ),
        },
        _ => {
            let value = json_input(&bytes, "export document")?;
            if !value.is_object() && !value.is_array() {
                return Err(AppError::invalid_input(
                    "export document must be a JSON object or array; no request was sent",
                ));
            }
            BoardImportRequest {
                options: Default::default(),
                board: value,
                members_mapping,
            }
        }
    };
    request.options = BoardImportOptions {
        split_by: args.split_by,
        members_mode: args.members_mode,
        csv_mapping,
        attachment_auth,
    };
    let body = PreparedImportRequest::new(&request)
        .map_err(|error| AppError::invalid_input(error.to_string()))?;
    let context = AuthenticatedContext::load(factory, store)?;
    let mut redactor = Redactor::with_secret(context.record().token());
    if let Some(token) = request
        .options
        .attachment_auth
        .as_ref()
        .and_then(|auth| auth.token.as_ref())
    {
        redactor = redactor.and_secret(&token.0);
    }
    let response = context
        .client()
        .import_board(source, body, context.record().token(), args.input.timeout)
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "board import",
                "the import target was not found",
            )
        })?;
    Ok(CommandSuccess::BoardImported(BoardImportSuccess {
        board_id: response.board_id,
        source: source.map_or("native", ImportSource::as_str).into(),
    }))
}
