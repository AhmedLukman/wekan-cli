use crate::{
    client::{WekanClientFactory, lists::TransferListRequest},
    command_result::{CommandSuccess, ListTransferSuccess},
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_client_error_with_not_found,
        transfer::{PositionArgs, identifier},
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct TransferArgs {
    /// Source list ID.
    #[arg(value_parser = identifier)]
    pub list_id: String,
    /// Source board ID.
    #[arg(long = "board", value_parser = identifier)]
    pub board_id: String,
    /// Destination board; defaults to the source board.
    #[arg(long = "target-board", value_parser = identifier)]
    pub target_board_id: Option<String>,
    /// Requested destination swimlane; omitted to use Wekan's behavior.
    #[arg(long, value_parser = identifier)]
    pub target_swimlane_id: Option<String>,
    #[command(flatten)]
    pub position: PositionArgs,
}
pub(super) async fn execute(
    args: TransferArgs,
    moving: bool,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let request = TransferListRequest {
        target_board_id: args.target_board_id,
        target_swimlane_id: args.target_swimlane_id,
        position: args.position.into_value(),
    };
    let context = AuthenticatedContext::load(factory, store)?;
    let result = if moving {
        context
            .client()
            .move_list(
                &args.board_id,
                &args.list_id,
                &request,
                context.record().token(),
            )
            .await
    } else {
        context
            .client()
            .copy_list(
                &args.board_id,
                &args.list_id,
                &request,
                context.record().token(),
            )
            .await
    }
    .map_err(|e| {
        map_client_error_with_not_found(
            e,
            &Redactor::with_secret(context.record().token()),
            "list transfer",
            "the source list or board scope was not found",
        )
    })?;
    let data = ListTransferSuccess {
        target_board_id: request
            .target_board_id
            .unwrap_or_else(|| args.board_id.clone()),
        target_swimlane_id: request.target_swimlane_id,
        source_board_id: args.board_id,
        source_list_id: args.list_id,
        returned_list_id: result,
    };
    Ok(if moving {
        CommandSuccess::ListMoved(data)
    } else {
        CommandSuccess::ListCopied(data)
    })
}
