//! Shared orchestration for native resource exports.
use std::{path::Path, time::Duration};

use super::{authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found};
use crate::{
    client::{
        WekanClientFactory,
        exports::{ExportError, ExportFormat, ExportOptions, ExportRequest, ExportTarget},
    },
    command_result::{CommandSuccess, ExportSuccess, StagedExport},
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    output::OutputFormat,
    redaction::Redactor,
};
use clap::Args;

pub const BOARD_FORMATS: [&str; 17] = [
    "json",
    "csv",
    "tsv",
    "xlsx",
    "pdf",
    "zip",
    "kanboard",
    "deck",
    "openproject",
    "github",
    "gitlab",
    "gitea",
    "forgejo",
    "trello",
    "jira",
    "asana",
    "zenkit",
];
pub const CARD_FIELDS: [&str; 16] = [
    "labels",
    "people",
    "board-info",
    "dates",
    "description",
    "custom-fields",
    "checklists",
    "subtasks",
    "comments",
    "attachments",
    "voting",
    "poker",
    "stickers",
    "locations",
    "dependencies",
    "sort",
];
pub const BOARD_FIELDS: [&str; 19] = [
    "card-details",
    "board-header",
    "activities",
    "labels",
    "people",
    "board-info",
    "dates",
    "description",
    "custom-fields",
    "checklists",
    "subtasks",
    "comments",
    "attachments",
    "voting",
    "poker",
    "stickers",
    "locations",
    "dependencies",
    "sort",
];

#[derive(Debug, Args)]
pub struct DeliveryArgs {
    /// Destination file path, or - for validated raw stdout.
    #[arg(long)]
    pub destination: std::path::PathBuf,
    /// Atomically replace an existing file after validation succeeds.
    #[arg(long)]
    pub overwrite: bool,
    /// Send the stored token as authToken instead of Bearer authentication.
    #[arg(long, conflicts_with = "no_auth")]
    pub auth_token_query: bool,
    /// Export without loading or sending a credential (for public boards).
    #[arg(long, conflicts_with = "auth_token_query")]
    pub no_auth: bool,
    /// Override the 30-second request timeout.
    #[arg(long, value_name = "SECONDS", value_parser = positive_timeout)]
    pub timeout: Option<Duration>,
}
impl DeliveryArgs {
    pub(crate) fn is_stdout(&self) -> bool {
        self.destination == Path::new("-")
    }
    pub(crate) fn validate(&self) -> Result<(), AppError> {
        if self.destination.as_os_str().is_empty() {
            return Err(AppError::invalid_input("--destination must not be empty"));
        }
        if self.is_stdout() && self.overwrite {
            return Err(AppError::invalid_input(
                "--overwrite cannot be used with --destination -",
            ));
        }
        Ok(())
    }
    pub(crate) fn validate_output(&self, output: OutputFormat) -> Result<(), AppError> {
        self.validate()?;
        if self.is_stdout() != (output == OutputFormat::Raw) {
            return Err(AppError::invalid_input(
                "--destination - requires --output raw; file destinations require human or json output",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Default, Args)]
pub struct LocaleArgs {
    /// Language code; Wekan may prefer the saved profile language.
    #[arg(long)]
    pub language: Option<String>,
    /// Requested native date format.
    #[arg(long, value_parser = ["YYYY-MM-DD", "DD-MM-YYYY", "MM-DD-YYYY"])]
    pub date_format: Option<String>,
    /// Native timezone string (at most 64 characters); empty means UTC.
    #[arg(long, value_parser = timezone)]
    pub timezone: Option<String>,
}
impl LocaleArgs {
    pub(crate) fn is_set(&self) -> bool {
        self.language.is_some() || self.date_format.is_some() || self.timezone.is_some()
    }
}
pub(crate) fn identifier(value: &str) -> Result<String, String> {
    super::transfer::identifier(value)
}
pub(crate) fn scope_id(value: &str) -> Result<String, String> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        Err(
            "export scope IDs must contain 1-64 ASCII letters, digits, underscores or hyphens"
                .into(),
        )
    } else {
        Ok(value.into())
    }
}
fn timezone(value: &str) -> Result<String, String> {
    if value.encode_utf16().count() > 64 {
        Err("timezone must contain at most 64 UTF-16 code units".into())
    } else {
        Ok(value.into())
    }
}
fn positive_timeout(value: &str) -> Result<Duration, String> {
    let seconds: u64 = value
        .parse()
        .map_err(|_| "timeout must be a positive integer number of seconds")?;
    if seconds == 0 {
        return Err("timeout must be greater than zero seconds".into());
    }
    Ok(Duration::from_secs(seconds))
}
pub(crate) fn file_error(operation: &str, error: std::io::Error) -> AppError {
    AppError::new(
        ErrorCode::FileIoError,
        format!(
            "could not {operation} export file: {}",
            crate::input::escape_terminal_text(&error.to_string())
        ),
        StableExitCode::Transport,
    )
}

pub(crate) async fn execute(
    target: ExportTarget,
    format: ExportFormat,
    request: ExportRequest,
    delivery: DeliveryArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    delivery.validate()?;
    let context = if delivery.no_auth {
        None
    } else {
        Some(AuthenticatedContext::load(factory, store)?)
    };
    let anonymous;
    let (client, token) = if let Some(context) = &context {
        (context.client(), Some(context.record().token()))
    } else {
        anonymous = factory.create()?;
        (&anonymous, None)
    };
    let mut staged = if delivery.is_stdout() {
        tempfile::NamedTempFile::new()
    } else {
        let parent = delivery
            .destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        tempfile::NamedTempFile::new_in(parent)
    }
    .map_err(|e| file_error("stage", e))?;
    let options = ExportOptions {
        format,
        request,
        token,
        query_token: delivery.auth_token_query,
        timeout: delivery.timeout,
    };
    let response = match &target {
        ExportTarget::Board { board_id } => {
            client
                .export_board(board_id, options, staged.as_file_mut())
                .await
        }
        ExportTarget::Card {
            board_id,
            list_id,
            card_id,
        } => {
            client
                .export_card(board_id, list_id, card_id, options, staged.as_file_mut())
                .await
        }
        ExportTarget::Attachment {
            board_id,
            attachment_id,
        } => {
            client
                .export_attachment(board_id, attachment_id, options, staged.as_file_mut())
                .await
        }
    }
    .map_err(|error| match error {
        ExportError::Client(error) => {
            let redactor = token.map(Redactor::with_secret).unwrap_or_default();
            map_client_error_with_not_found(
                error,
                &redactor,
                "export",
                "the export board or scoped resource was not found",
            )
        }
        ExportError::Io(error) => file_error("stage or validate", error),
    })?;
    let mut result = ExportSuccess::new(
        target,
        format,
        delivery.destination.display().to_string(),
        response,
    );
    if delivery.is_stdout() {
        result.staged = Some(StagedExport(staged));
    } else {
        staged
            .as_file()
            .sync_all()
            .map_err(|e| file_error("sync", e))?;
        let persisted = if delivery.overwrite {
            staged.persist(&delivery.destination)
        } else {
            staged.persist_noclobber(&delivery.destination)
        };
        persisted.map_err(|e| {
            file_error(
                "save (use --overwrite to replace an existing destination)",
                e.error,
            )
        })?;
        result.file_saved = true;
    }
    Ok(CommandSuccess::Exported(result))
}
