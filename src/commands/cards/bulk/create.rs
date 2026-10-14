use crate::{
    client::{
        WekanClientFactory,
        cards::bulk::{BulkCreateCardInput, BulkCreateCardsRequest},
    },
    command_result::{BulkCardCreateItem, BulkCardCreateResult, CardBatchResult, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_mutation_client_error_with_not_found, custom_fields::non_empty,
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{BinaryInputError, PayloadSource},
    redaction::Redactor,
};
use clap::{ArgGroup, Args};
use std::path::PathBuf;

const MAX_JSON_BYTES: usize = 50 * 1024 * 1024;

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("cards_source").args(["cards", "cards_file", "cards_stdin"]).required(true)))]
pub struct CreateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    board_id: String,
    /// Wekan list ID.
    #[arg(long = "list", value_parser = non_empty)]
    list_id: String,
    /// Default swimlane; a card's nonempty swimlaneId takes precedence.
    #[arg(long, value_parser = super::body_id)]
    swimlane_id: Option<String>,
    /// JSON card array using title, description, parentId, swimlaneId, members, assignees (50 MiB limit).
    #[arg(long, value_name = "JSON")]
    cards: Option<String>,
    /// Read the UTF-8 JSON card array from a regular file (50 MiB limit).
    #[arg(long, value_name = "PATH")]
    cards_file: Option<PathBuf>,
    /// Read the UTF-8 JSON card array from standard input (50 MiB limit).
    #[arg(long)]
    cards_stdin: bool,
}

fn input_error(error: BinaryInputError) -> AppError {
    match error {
        BinaryInputError::TooLarge => AppError::invalid_input(
            "bulk card JSON input exceeds the 50 MiB client limit; no request was sent",
        ),
        BinaryInputError::NotRegularFile => AppError::invalid_input(
            "bulk card JSON file input must be a regular file; no request was sent",
        ),
        BinaryInputError::Io { .. } => AppError::new(
            ErrorCode::FileIoError,
            "could not read the bulk card JSON input",
            StableExitCode::Transport,
        ),
    }
}

pub(super) async fn execute(
    args: CreateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let source = if let Some(cards) = args.cards {
        PayloadSource::Text(cards)
    } else if let Some(path) = args.cards_file {
        PayloadSource::File(path)
    } else {
        PayloadSource::Stdin
    };
    let bytes = source
        .acquire_bounded(MAX_JSON_BYTES)
        .await
        .map_err(input_error)?;
    let cards: Vec<BulkCreateCardInput> = serde_json::from_slice(&bytes).map_err(|error| AppError::invalid_input(format!(
        "expected a typed JSON card array with supported camelCase properties at line {}, column {}; no request was sent", error.line(), error.column())))?;
    super::validate_count(cards.len())?;
    let request = BulkCreateCardsRequest {
        swimlane_id: args.swimlane_id,
        cards,
    };
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let results = context
        .client()
        .bulk_create_cards(
            &args.board_id,
            &args.list_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_mutation_client_error_with_not_found(
                error,
                &redactor,
                "bulk card creation",
                "the requested Wekan board or list was not found",
            )
        })?;
    super::finish(CardBatchResult::CardBulkCreate(BulkCardCreateResult {
        board_id: args.board_id,
        list_id: args.list_id,
        results: results
            .into_iter()
            .map(|item| BulkCardCreateItem {
                index: item.index,
                card_id: item.card_id,
                error: item.error.map(|error| redactor.redact(&error)),
            })
            .collect(),
    }))
}
