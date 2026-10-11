use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{CardRestoreSuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_mutation_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
#[command(after_help = "Example:\n  wekan card restore card-id --board board-id --list list-id")]
pub struct RestoreArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// Wekan list ID. Must match the archived card's list.
    #[arg(long = "list", value_parser = non_empty, help_heading = "Target")]
    pub list_id: String,

    /// Wekan archived card ID. Child cards are restored recursively.
    #[arg(value_parser = non_empty, help_heading = "Target")]
    pub card_id: String,
}

pub(super) async fn execute(
    args: RestoreArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let restored = context
        .client()
        .restore_card(
            &args.board_id,
            &args.list_id,
            &args.card_id,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_mutation_client_error_with_not_found(
                error,
                &redactor,
                "card restore",
                "the requested Wekan card or board was not found",
            )
        })?;
    Ok(CommandSuccess::CardRestored(CardRestoreSuccess {
        board_id: args.board_id,
        list_id: args.list_id,
        card_id: restored.card_id,
        archived: restored.archived,
    }))
}
