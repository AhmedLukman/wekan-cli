use crate::{
    client::{WekanClientFactory, swimlanes::TransferSwimlaneRequest},
    command_result::{CommandSuccess, SwimlaneTransferSuccess},
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
    /// Source swimlane ID.
    #[arg(value_parser = identifier)]
    pub swimlane_id: String,
    /// Source board ID.
    #[arg(long = "board", value_parser = identifier)]
    pub board_id: String,
    /// Destination board; defaults to the source board.
    #[arg(long = "target-board", value_parser = identifier)]
    pub target_board_id: Option<String>,
    /// Requested title, sent unchanged; empty/blank uses the source title.
    #[arg(long)]
    pub title: Option<String>,
    #[command(flatten)]
    pub position: PositionArgs,
}
pub(super) async fn execute(
    args: TransferArgs,
    moving: bool,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let request = TransferSwimlaneRequest {
        target_board_id: args.target_board_id,
        title: args.title,
        position: args.position.into_value(),
    };
    let context = AuthenticatedContext::load(factory, store)?;
    let result = if moving {
        context
            .client()
            .move_swimlane(
                &args.board_id,
                &args.swimlane_id,
                &request,
                context.record().token(),
            )
            .await
    } else {
        context
            .client()
            .copy_swimlane(
                &args.board_id,
                &args.swimlane_id,
                &request,
                context.record().token(),
            )
            .await
    }
    .map_err(|e| {
        map_client_error_with_not_found(
            e,
            &Redactor::with_secret(context.record().token()),
            "swimlane transfer",
            "the source swimlane or board scope was not found",
        )
    })?;
    let data = SwimlaneTransferSuccess {
        target_board_id: request
            .target_board_id
            .unwrap_or_else(|| args.board_id.clone()),

        source_board_id: args.board_id,
        source_swimlane_id: args.swimlane_id,
        returned_swimlane_id: result,
    };
    Ok(if moving {
        CommandSuccess::SwimlaneMoved(data)
    } else {
        CommandSuccess::SwimlaneCopied(data)
    })
}
