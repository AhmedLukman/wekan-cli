use crate::{
    client::{
        WekanClientFactory,
        boards::import::{NewBoardZipImportTarget, ZipImportTarget},
    },
    command_result::{BoardImportSuccess, CommandSuccess, ZipImportSuccess},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_client_error_with_not_found,
        exports,
        imports::{self, ImportInput},
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct ImportZipArgs {
    #[command(flatten)]
    pub input: ImportInput,
    /// Existing destination board.
    #[arg(long = "board", value_parser = exports::identifier, required_unless_present = "new_board", conflicts_with = "new_board")]
    pub board_id: Option<String>,
    /// Create a new board from the archive.
    #[arg(long, conflicts_with_all = ["board_id", "swimlane", "list", "card", "checklist"])]
    pub new_board: bool,
    /// Native imported-member handling for a new board.
    #[arg(long, value_parser = ["map", "placeholder", "me"], requires = "new_board", conflicts_with = "board_id")]
    pub members_mode: Option<String>,
    /// Native section selection. Repeat for multiple sections.
    #[arg(long = "field", value_parser = exports::BOARD_FIELDS)]
    pub fields: Vec<String>,
    #[arg(long, value_parser = exports::scope_id)]
    pub swimlane: Option<String>,
    #[arg(long, value_parser = exports::scope_id)]
    pub list: Option<String>,
    #[arg(long, value_parser = exports::scope_id)]
    pub card: Option<String>,
    /// Sent to Wekan, but ignored by its v12.57 importer.
    #[arg(long, value_parser = exports::scope_id)]
    pub checklist: Option<String>,
    /// Send the stored token as authToken instead of Bearer authentication.
    #[arg(long)]
    pub auth_token_query: bool,
}

pub(crate) async fn execute(
    args: ImportZipArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let input = imports::acquire_zip(&args.input.file).await?;
    if args.new_board {
        let response = context
            .client()
            .import_zip_new_board(
                &NewBoardZipImportTarget {
                    fields: args.fields,
                    members_mode: args.members_mode,
                },
                input.reader,
                input.length,
                context.record().token(),
                args.auth_token_query,
                args.input.timeout,
            )
            .await
            .map_err(|error| {
                map_client_error_with_not_found(
                    error,
                    &Redactor::with_secret(context.record().token()),
                    "ZIP new-board import",
                    "the import target was not found",
                )
            })?;
        return Ok(CommandSuccess::BoardImported(BoardImportSuccess {
            board_id: response.board_id,
            source: "wekan-zip".into(),
        }));
    }
    let target = ZipImportTarget {
        board_id: args.board_id.expect("parser requires a destination board"),
        fields: args.fields,
        swimlane_id: args.swimlane,
        list_id: args.list,
        card_id: args.card,
        checklist_id: args.checklist,
    };
    let response = context
        .client()
        .import_zip(
            &target,
            input.reader,
            input.length,
            context.record().token(),
            args.auth_token_query,
            args.input.timeout,
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &Redactor::with_secret(context.record().token()),
                "ZIP import",
                "the destination board was not found",
            )
        })?;
    Ok(CommandSuccess::ZipImported(ZipImportSuccess {
        board_id: target.board_id,
        swimlane_id: target.swimlane_id,
        list_id: target.list_id,
        card_id: target.card_id,
        checklist_id: target.checklist_id,
        counts: response.counts,
    }))
}
