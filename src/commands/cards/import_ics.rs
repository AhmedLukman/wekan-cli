use crate::{
    client::{
        ClientError, WekanClientFactory, cards::import::IcsImportRequest,
        imports::PreparedImportRequest,
    },
    command_result::{CommandSuccess, IcsImportSuccess},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_client_error_with_not_found, server_error_details},
        exports,
        imports::{self, ImportInput},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct ImportIcsArgs {
    #[command(flatten)]
    pub input: ImportInput,
    #[arg(long = "board", value_parser = exports::identifier)]
    pub board_id: String,
    #[arg(long, value_parser = exports::identifier)]
    pub swimlane_id: String,
    #[arg(long = "list", value_parser = exports::identifier)]
    pub list_id: String,
}

pub(crate) async fn execute(
    args: ImportIcsArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let text = String::from_utf8(imports::acquire(&args.input.file).await?).map_err(|_| {
        AppError::invalid_input("calendar input must be UTF-8; no request was sent")
    })?;
    let body = PreparedImportRequest::new(&IcsImportRequest { ics: text })
        .map_err(|error| AppError::invalid_input(error.to_string()))?;
    let context = AuthenticatedContext::load(factory, store)?;
    let response = context
        .client()
        .import_ics(
            &args.board_id,
            &args.swimlane_id,
            &args.list_id,
            body,
            context.record().token(),
            args.input.timeout,
        )
        .await
        .map_err(|error| map_error(error, &Redactor::with_secret(context.record().token())))?;
    Ok(CommandSuccess::IcsImported(IcsImportSuccess {
        board_id: args.board_id,
        swimlane_id: args.swimlane_id,
        list_id: args.list_id,
        created: response.created,
        card_ids: response.card_ids,
    }))
}

fn map_error(error: ClientError, redactor: &Redactor<'_>) -> AppError {
    // v12.57 checks login first, then uses this exact 401 for missing boards
    // and callers without board write access (server/methods/icsImport.js).
    if let ClientError::Server {
        status,
        server_error,
        server_reason,
        retry_after_seconds,
    } = &error
        && *status == reqwest::StatusCode::UNAUTHORIZED
        && server_error.as_deref() == Some("not-authorized")
    {
        return AppError::new(
            ErrorCode::PermissionDenied,
            "Wekan denied access to the calendar destination",
            StableExitCode::Server,
        )
        .with_details(server_error_details(
            *status,
            server_error.clone(),
            server_reason.clone(),
            *retry_after_seconds,
            redactor,
        ));
    }
    map_client_error_with_not_found(
        error,
        redactor,
        "ICS import",
        "the calendar destination was not found",
    )
}
