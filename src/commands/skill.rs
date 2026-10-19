mod list;
mod show;
mod topics;

use clap::{Args, Subcommand};

use crate::{command_result::CommandSuccess, error::AppError};

pub use self::{list::ListArgs, show::ShowArgs};

#[derive(Debug, Args)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub command: SkillCommand,
}

#[derive(Debug, Subcommand)]
pub enum SkillCommand {
    /// List the available embedded guidance topics alphabetically.
    List(ListArgs),
    /// Print an embedded Markdown guide; defaults to core.
    Show(ShowArgs),
}

pub(crate) fn dispatch(command: SkillCommand) -> Result<CommandSuccess, AppError> {
    match command {
        SkillCommand::List(args) => Ok(list::execute(args)),
        SkillCommand::Show(args) => show::execute(args),
    }
}
