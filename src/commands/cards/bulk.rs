mod create;
mod delete;
mod labels;

use crate::{
    client::WekanClientFactory,
    command_result::{CardBatchResult, CommandSuccess},
    credentials::CredentialStore,
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    input::ConfirmationProvider,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct BulkArgs {
    #[command(subcommand)]
    pub command: BulkCommand,
}
#[derive(Debug, Subcommand)]
pub enum BulkCommand {
    /// Create 1-500 cards from a JSON array in one request.
    Create(create::CreateArgs),
    /// Permanently delete 1-500 requested cards; Wekan v11.06 retains attachments.
    Delete(delete::DeleteArgs),
    /// Merge label additions/removals on 1-500 active cards.
    Labels(labels::LabelsArgs),
}

#[derive(Debug, Args)]
pub struct Target {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = crate::commands::custom_fields::non_empty)]
    pub board_id: String,
    /// Card ID; repeat in caller order. Duplicate IDs are preserved.
    #[arg(long = "card", required = true, value_parser = body_id)]
    pub card_ids: Vec<String>,
}

fn body_id(value: &str) -> Result<String, String> {
    if value.is_empty() {
        Err("identifier must not be empty".into())
    } else {
        Ok(value.to_owned())
    }
}

fn validate_count(count: usize) -> Result<(), AppError> {
    if !(1..=500).contains(&count) {
        return Err(AppError::invalid_input(
            "bulk operations require 1-500 cards; no request was sent",
        ));
    }
    Ok(())
}

fn finish(batch: CardBatchResult) -> Result<CommandSuccess, AppError> {
    if batch.incomplete() {
        return Err(AppError::new(ErrorCode::BatchIncomplete,
            "Wekan completed the batch with failed items or unmatched card IDs; inspect batch_result before retrying",
            StableExitCode::Server).with_details(ErrorDetails {
                batch_result: Some(batch), http_status: Some(200), http_success: Some(true), retry_safe: Some(false),
                ..ErrorDetails::default()
            }));
    }
    Ok(CommandSuccess::CardBatch(batch))
}

pub(super) async fn dispatch(
    command: BulkCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        BulkCommand::Create(args) => create::execute(args, factory, store).await,
        BulkCommand::Delete(args) => delete::execute(args, factory, store, confirmation).await,
        BulkCommand::Labels(args) => labels::execute(args, factory, store).await,
    }
}
