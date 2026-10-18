use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{
    client::{
        WekanClientFactory,
        boards::{
            import::BoardImportRequest,
            trello_import::{TrelloImportAuth, TrelloImportBoardName, TrelloImportFormat},
        },
        imports::PreparedImportRequest,
    },
    command_result::{CommandSuccess, TrelloImportResult},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_client_error_with_not_found,
        imports::{self, ImportInput},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct ImportTrelloArgs {
    #[command(flatten)]
    pub input: ImportInput,
    /// Explicit input representation; no filename detection is performed.
    #[arg(long, value_parser = ["json", "zip"])]
    pub format: String,
    /// JSON object mapping Trello member IDs to Wekan user IDs. JSON imports only.
    #[arg(long, value_name = "JSON", conflicts_with = "members_mapping_file")]
    pub members_mapping: Option<String>,
    /// Read member mapping from a regular UTF-8 JSON file (50 MiB). JSON imports only.
    #[arg(long, value_name = "PATH")]
    pub members_mapping_file: Option<PathBuf>,
    /// Authentication form using the stored login token.
    #[arg(long, value_parser = ["bearer", "x-auth-token", "meteor-cookie", "wekan-cookie"], default_value = "bearer")]
    pub auth: String,
}

impl ImportTrelloArgs {
    pub(crate) fn validate(&self) -> Result<(), AppError> {
        if self.format == "zip"
            && (self.members_mapping.is_some() || self.members_mapping_file.is_some())
        {
            return Err(AppError::invalid_input(
                "member mapping requires --format json; no request was sent",
            ));
        }
        if self.members_mapping_file.as_deref() == Some(Path::new("-")) {
            return Err(AppError::invalid_input(
                "--members-mapping-file requires a regular file; stdin is reserved for the board",
            ));
        }
        Ok(())
    }
}

enum PreparedTrelloInput {
    Json(PreparedImportRequest<BoardImportRequest>),
    Zip(crate::input::BinaryStreamInput),
}

pub(crate) async fn execute(
    args: ImportTrelloArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    args.validate()?;
    let format = if args.format == "json" {
        TrelloImportFormat::Json
    } else {
        TrelloImportFormat::Zip
    };
    let auth = match args.auth.as_str() {
        "x-auth-token" => TrelloImportAuth::XAuthToken,
        "meteor-cookie" => TrelloImportAuth::MeteorCookie,
        "wekan-cookie" => TrelloImportAuth::WekanCookie,
        _ => TrelloImportAuth::Bearer,
    };
    // Acquire and validate everything before loading credentials or sending HTTP.
    let input = match format {
        TrelloImportFormat::Json => {
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
            let board = serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|_| {
                AppError::invalid_input(
                    "expected a UTF-8 JSON Trello board object; no request was sent",
                )
            })?;
            if !board.is_object() {
                return Err(AppError::invalid_input(
                    "Trello board must be a JSON object; no request was sent",
                ));
            }
            let body = PreparedImportRequest::new(&BoardImportRequest {
                board,
                members_mapping,
            })
            .map_err(|error| AppError::invalid_input(error.to_string()))?;
            PreparedTrelloInput::Json(body)
        }
        TrelloImportFormat::Zip => {
            PreparedTrelloInput::Zip(imports::acquire_trello_zip(&args.input.file).await?)
        }
    };
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let response = match input {
        PreparedTrelloInput::Json(body) => {
            context
                .client()
                .import_trello_json(body, context.record().token(), auth, args.input.timeout)
                .await
        }
        PreparedTrelloInput::Zip(input) => {
            context
                .client()
                .import_trello_zip(
                    input.reader,
                    input.length,
                    context.record().token(),
                    auth,
                    args.input.timeout,
                )
                .await
        }
    }
    .map_err(|error| {
        let error = map_client_error_with_not_found(
            error,
            &redactor,
            "Trello import",
            "the Trello import endpoint was not found",
        );
        if !matches!(
            error.code(),
            ErrorCode::AuthenticationRejected | ErrorCode::InternalError
        ) {
            let mut details = error.details().clone();
            details.retry_safe = Some(false);
            details.mutation_attempted = Some(true);
            error.with_details(details)
        } else {
            error
        }
    })?;
    let mut result = TrelloImportResult {
        format,
        board_ids: response.board_ids,
        errors: response.errors,
    };
    if let Some(errors) = &mut result.errors {
        for error in errors {
            if let TrelloImportBoardName::String(name) = &mut error.board {
                *name = redactor.redact(name);
            }
            error.error = redactor.redact(&error.error);
        }
    }
    if result
        .errors
        .as_ref()
        .is_some_and(|errors| !errors.is_empty())
    {
        return Err(AppError::new(
            ErrorCode::BatchIncomplete,
            "Wekan reported failed Trello boards; inspect trello_import_result before retrying",
            StableExitCode::Server,
        )
        .with_details(ErrorDetails {
            trello_import_result: Some(result),
            http_status: Some(200),
            http_success: Some(true),
            mutation_attempted: Some(true),

            retry_safe: Some(false),
            ..Default::default()
        }));
    }
    Ok(CommandSuccess::TrelloImported(result))
}
