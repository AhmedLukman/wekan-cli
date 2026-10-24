use crate::{
    client::{
        WekanClientFactory,
        exports::{ChartExportOptions, ExportRequest, ExportTarget},
    },
    command_result::CommandSuccess,
    commands::exports::{self, DeliveryArgs, LocaleArgs},
    credentials::CredentialStore,
    error::AppError,
};
use clap::{Args, Subcommand};
pub const CHART_KEYS: [&str; 21] = [
    "dashboard",
    "burndown",
    "burnup",
    "cumulativeFlow",
    "controlChart",
    "cycleTime",
    "flowEfficiency",
    "leadTime",
    "throughputHistogram",
    "wipRun",
    "gantt",
    "time",
    "pulse",
    "scrumVelocity",
    "scrumSprint",
    "scrumDaily",
    "agingWip",
    "blockerAnalysis",
    "monteCarlo",
    "processBehavior",
    "sizeCycleTime",
];
#[derive(Debug, Args)]
pub struct ChartArgs {
    #[command(subcommand)]
    pub command: ChartCommand,
}
impl ChartArgs {
    pub(crate) fn delivery(&self) -> Option<&DeliveryArgs> {
        let ChartCommand::Export(args) = &self.command;
        Some(&args.delivery)
    }
}
#[derive(Debug, Subcommand)]
pub enum ChartCommand {
    /// Export a board chart as a native PDF or Excel workbook.
    Export(Box<ChartExportArgs>),
}
#[derive(Debug, Args)]
pub struct ChartExportArgs {
    #[arg(value_parser=CHART_KEYS)]
    pub chart: String,
    #[arg(long,value_parser=exports::identifier)]
    pub board: String,
    #[arg(long,value_parser=["pdf","xlsx"])]
    pub format: String,
    /// Required by the native Scrum Sprint and Scrum Daily chart types.
    #[arg(long)]
    pub sprint: Option<String>,
    #[arg(long,value_parser=clap::value_parser!(u32).range(1..=10000))]
    pub target_count: Option<u32>,
    #[arg(long,value_parser=clap::value_parser!(u32).range(7..=365))]
    pub history_days: Option<u32>,
    /// Monte Carlo target date, YYYY-MM-DD.
    #[arg(long)]
    pub target_date: Option<String>,
    /// Poker or a numeric custom-field ID.
    #[arg(long)]
    pub size_field: Option<String>,
    #[command(flatten)]
    pub locale: LocaleArgs,
    #[command(flatten)]
    pub delivery: DeliveryArgs,
}
pub(crate) async fn dispatch(
    command: ChartCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let ChartCommand::Export(args) = command;
    if matches!(args.chart.as_str(), "scrumSprint" | "scrumDaily") && args.sprint.is_none() {
        return Err(AppError::invalid_input(
            "this chart requires --sprint; no request was sent",
        ));
    }
    let request = ExportRequest {
        chart: Some(ChartExportOptions {
            sprint_id: args.sprint,
            target_count: args.target_count,
            history_days: args.history_days,
            target_date: args.target_date,
            size_field: args.size_field,
        }),
        language: args.locale.language,
        date_format: args.locale.date_format,
        timezone: args.locale.timezone,
        ..Default::default()
    };
    exports::execute(
        ExportTarget::Chart {
            board_id: args.board,
            chart_key: args.chart,
        },
        args.format.parse().expect("validated format"),
        request,
        args.delivery,
        factory,
        store,
    )
    .await
}
