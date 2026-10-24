use clap::{Args, Subcommand};

use crate::{
    command_result::{CancellationSuccess, CommandSuccess, DestructiveOperation},
    error::AppError,
    input::ConfirmationProvider,
    update::{self, UpdateService},
};

#[derive(Debug, Args)]
pub struct SelfArgs {
    #[command(subcommand)]
    pub command: SelfCommand,
}

#[derive(Debug, Subcommand)]
pub enum SelfCommand {
    /// Check for and install a stable Wekan CLI update using the original installation method.
    Update(UpdateArgs),
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Check for a newer stable version without installing it.
    #[arg(long, conflicts_with = "yes")]
    pub check: bool,
    /// Install an available update without prompting.
    #[arg(long)]
    pub yes: bool,
}

pub(crate) fn dispatch(
    command: SelfCommand,
    service: UpdateService,
    confirmation: &dyn ConfirmationProvider,
) -> impl std::future::Future<Output = Result<CommandSuccess, AppError>> + Send + '_ {
    Box::pin(async move {
        let SelfCommand::Update(args) = command;
        match service.manual(args.check, args.yes, confirmation).await? {
            update::ManualOutcome::Complete(data) => Ok(CommandSuccess::SelfUpdate(data)),
            update::ManualOutcome::Cancelled => Ok(CommandSuccess::Cancelled(
                CancellationSuccess::new(DestructiveOperation::SelfUpdate),
            )),
        }
    })
}
