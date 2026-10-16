use crate::commands::exports::{self, DeliveryArgs};
use crate::{
    client::{
        WekanClientFactory,
        exports::{ExportFormat, ExportRequest, ExportTarget},
    },
    command_result::CommandSuccess,
    credentials::CredentialStore,
    error::AppError,
};
use clap::Args;
#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Attachment to export as native JSON (not the decoded file).
    #[arg(value_parser=exports::identifier)]
    pub attachment_id: String,
    /// Parent board. Missing attachment IDs natively export {}.
    #[arg(long="board",value_parser=exports::identifier)]
    pub board_id: String,
    #[command(flatten)]
    pub delivery: DeliveryArgs,
}
pub(crate) async fn execute(
    args: ExportArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    exports::execute(
        ExportTarget::Attachment {
            board_id: args.board_id,
            attachment_id: args.attachment_id,
        },
        ExportFormat::Json,
        ExportRequest::default(),
        args.delivery,
        factory,
        store,
    )
    .await
}
