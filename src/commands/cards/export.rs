use crate::commands::exports::{self, DeliveryArgs, LocaleArgs};
use crate::{
    client::{
        WekanClientFactory,
        exports::{ExportRequest, ExportTarget},
    },
    command_result::CommandSuccess,
    credentials::CredentialStore,
    error::AppError,
};
use clap::Args;
#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Card to export.
    #[arg(value_parser=exports::identifier)]
    pub card_id: String,
    /// Parent board.
    #[arg(long="board",value_parser=exports::identifier)]
    pub board_id: String,
    /// Parent list.
    #[arg(long="list",value_parser=exports::identifier)]
    pub list_id: String,
    /// Native card artifact format.
    #[arg(long,value_parser=["xlsx","pdf"])]
    pub format: String,
    /// Native card section; repeat to select multiple sections.
    #[arg(long="field",value_parser=exports::CARD_FIELDS)]
    pub fields: Vec<String>,
    #[command(flatten)]
    pub locale: LocaleArgs,
    #[command(flatten)]
    pub delivery: DeliveryArgs,
}
pub(crate) async fn execute(
    args: ExportArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    args.delivery.validate()?;
    let request = ExportRequest {
        fields: args.fields,
        language: args.locale.language,
        date_format: args.locale.date_format,
        timezone: args.locale.timezone,
        ..ExportRequest::default()
    };
    exports::execute(
        ExportTarget::Card {
            board_id: args.board_id,
            list_id: args.list_id,
            card_id: args.card_id,
        },
        args.format.parse().expect("validated card format"),
        request,
        args.delivery,
        factory,
        store,
    )
    .await
}
