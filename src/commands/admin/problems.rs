use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct ProblemArgs {
    #[command(subcommand)]
    pub command: ProblemCommand,
}
#[derive(Debug, Subcommand)]
pub enum ProblemCommand {
    /// Read the status overview and counts of new problems.
    List,
    /// Read a page of one event stream.
    Get(GetArgs),
    /// Mark the current problems in a stream as seen.
    Acknowledge(AcknowledgeArgs),
}
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Event stream, or api for API usage.
    #[arg(value_parser=["security","speed","tests","cpu","ram","disk","database","integrity","mail","api"])]
    stream: String,
    /// Rows per page, from 1 through 200.
    #[arg(long,default_value_t=50,value_parser=clap::value_parser!(u64).range(1..=200))]
    limit: u64,
    /// Number of matching rows to skip.
    #[arg(long,default_value_t=0,value_parser=clap::value_parser!(u64).range(0..=9_007_199_254_740_991))]
    skip: u64,
    /// Case-insensitive text filter.
    #[arg(long)]
    search: Option<String>,
}
#[derive(Debug, Args)]
pub struct AcknowledgeArgs {
    /// Problem stream to acknowledge; API usage is read-only.
    #[arg(value_parser=crate::client::admin::problems::PROBLEM_STREAMS)]
    stream: String,
}
pub(crate) async fn execute(
    command: ProblemCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    match command {
        ProblemCommand::List => context
            .client()
            .problems_overview(context.record().token())
            .await
            .map(|response| CommandSuccess::ProblemsOverview(response.into())),
        ProblemCommand::Get(args) => context
            .client()
            .problem_stream(
                &args.stream,
                args.limit,
                args.skip,
                args.search.as_deref(),
                context.record().token(),
            )
            .await
            .map(|response| CommandSuccess::ProblemStream(response.into())),
        ProblemCommand::Acknowledge(args) => context
            .client()
            .acknowledge_problem_stream(&args.stream, context.record().token())
            .await
            .map(|response| CommandSuccess::ProblemAcknowledged(response.into())),
    }
    .map_err(|e| map_client_error(e, &redactor, "admin problems"))
}
