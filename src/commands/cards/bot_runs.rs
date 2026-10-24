use crate::{
    client::{WekanClientFactory, cards::bot_runs::BotRunUpdate},
    command_result::CommandSuccess,
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct BotRunArgs {
    #[command(subcommand)]
    pub command: BotRunCommand,
}
#[derive(Debug, Subcommand)]
pub enum BotRunCommand {
    /// Report native bot run progress, questions or completion.
    Report(ReportArgs),
}
#[derive(Debug, Args)]
pub struct ReportArgs {
    /// Native bot run ID.
    #[arg(value_parser = super::trimmed_non_empty)]
    run_id: String,
    #[arg(long, value_parser = super::trimmed_non_empty)]
    board: String,
    #[arg(long, value_parser = super::trimmed_non_empty)]
    card: String,
    /// Typed JSON report file, or - for UTF-8 stdin.
    #[arg(long)]
    file: PathBuf,
}
pub(crate) async fn dispatch(
    command: BotRunCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let BotRunCommand::Report(args) = command;
    let bytes = crate::commands::imports::acquire(&args.file).await?;
    let update: BotRunUpdate = serde_json::from_slice(&bytes).map_err(|_| {
        AppError::invalid_input("expected a typed bot report JSON object; no request was sent")
    })?;
    let context = AuthenticatedContext::load(factory, store)?;
    let response = context
        .client()
        .report_bot_run(
            &args.board,
            &args.card,
            &args.run_id,
            &update,
            context.record().token(),
        )
        .await
        .map_err(|e| {
            map_client_error(
                e,
                &Redactor::with_secret(context.record().token()),
                "bot run report",
            )
        })?;
    Ok(CommandSuccess::BotRunReported(response.into()))
}
