use crate::commands::exports::{self, DeliveryArgs, LocaleArgs};
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
    /// Board to export.
    #[arg(value_parser = exports::identifier)]
    pub board_id: String,
    /// Native artifact format; never inferred from the destination extension.
    #[arg(long,value_parser = exports::BOARD_FORMATS)]
    pub format: String,
    /// Native section to include. Repeat to select multiple sections.
    #[arg(long = "field",value_parser = exports::BOARD_FIELDS)]
    pub fields: Vec<String>,
    /// Swimlane scope (JSON, XLSX, PDF, ZIP).
    #[arg(long,value_parser = exports::scope_id)]
    pub swimlane: Option<String>,
    /// List scope (JSON, XLSX, PDF, ZIP).
    #[arg(long,value_parser = exports::scope_id)]
    pub list: Option<String>,
    /// Card scope (JSON, XLSX, PDF, ZIP).
    #[arg(long,value_parser = exports::scope_id)]
    pub card: Option<String>,
    /// Checklist scope (JSON, XLSX, PDF, ZIP).
    #[arg(long,value_parser = exports::scope_id)]
    pub checklist: Option<String>,
    /// Include base64 attachment data in JSON exports.
    #[arg(long,action=clap::ArgAction::Set)]
    pub attachments: Option<bool>,
    /// CSV delimiter; TSV always uses a tab.
    #[arg(long)]
    pub delimiter: Option<String>,
    #[command(flatten)]
    pub locale: LocaleArgs,
    #[command(flatten)]
    pub delivery: DeliveryArgs,
}
impl ExportArgs {
    pub(crate) fn validate(&self) -> Result<(), AppError> {
        self.delivery.validate()?;
        let scoped = self.swimlane.is_some()
            || self.list.is_some()
            || self.card.is_some()
            || self.checklist.is_some();
        if scoped && !matches!(self.format.as_str(), "json" | "xlsx" | "pdf" | "zip") {
            return Err(AppError::invalid_input(
                "scope selectors require json, xlsx, pdf or zip format",
            ));
        }
        if self.attachments.is_some() && self.format != "json" {
            return Err(AppError::invalid_input(
                "--attachments requires json format",
            ));
        }
        if self.delimiter.is_some() && self.format != "csv" {
            return Err(AppError::invalid_input("--delimiter requires csv format"));
        }
        if self.locale.is_set() && !matches!(self.format.as_str(), "xlsx" | "pdf") {
            return Err(AppError::invalid_input(
                "locale options require xlsx or pdf format",
            ));
        }
        if !self.fields.is_empty() && self.format == "kanboard" {
            return Err(AppError::invalid_input("kanboard does not support --field"));
        }
        Ok(())
    }
}
pub(crate) async fn execute(
    args: ExportArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    args.validate()?;
    let request = ExportRequest {
        fields: args.fields,
        swimlane_id: args.swimlane,
        list_id: args.list,
        card_id: args.card,
        checklist_id: args.checklist,
        attachments: args.attachments,
        delimiter: args.delimiter,
        language: args.locale.language,
        date_format: args.locale.date_format,
        timezone: args.locale.timezone,
    };
    let format: ExportFormat = args.format.parse().expect("validated export format");
    exports::execute(
        ExportTarget::Board {
            board_id: args.board_id,
        },
        format,
        request,
        args.delivery,
        factory,
        store,
    )
    .await
}
