use crate::{
    client::{WekanClientFactory, cards::CopyCardRequest},
    command_result::{CardCopySuccess, CommandSuccess},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_client_error_with_not_found,
        transfer::{CardPositionArgs, identifier},
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct CopyArgs {
    /// Source card ID.
    #[arg(value_parser = identifier)]
    pub card_id: String,
    /// Source board ID.
    #[arg(long = "board", value_parser = identifier)]
    pub board_id: String,
    /// Source list ID.
    #[arg(long = "list", value_parser = identifier)]
    pub list_id: String,
    /// Destination board; defaults to the source board.
    #[arg(long = "target-board", value_parser = identifier)]
    pub target_board_id: Option<String>,
    /// Destination list; defaults to the source list, including cross-board copies.
    #[arg(long = "target-list", value_parser = identifier)]
    pub target_list_id: Option<String>,
    /// Required destination swimlane ID.
    #[arg(long, value_parser = identifier)]
    pub target_swimlane_id: String,
    #[command(flatten)]
    pub position: CardPositionArgs,
}
pub(super) async fn execute(
    args: CopyArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let request = CopyCardRequest {
        target_board_id: args.target_board_id,
        target_list_id: args.target_list_id,
        target_swimlane_id: args.target_swimlane_id,
        position: args.position.into_value(),
    };
    let context = AuthenticatedContext::load(factory, store)?;
    let card_id = context
        .client()
        .copy_card(
            &args.board_id,
            &args.list_id,
            &args.card_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &Redactor::with_secret(context.record().token()),
                "card copy",
                "the source card or board scope was not found",
            )
        })?;
    Ok(CommandSuccess::CardCopied(CardCopySuccess {
        target_board_id: request
            .target_board_id
            .unwrap_or_else(|| args.board_id.clone()),
        target_list_id: request
            .target_list_id
            .unwrap_or_else(|| args.list_id.clone()),
        target_swimlane_id: request.target_swimlane_id,
        source_board_id: args.board_id,
        source_list_id: args.list_id,
        source_card_id: args.card_id,
        card_id,
    }))
}
