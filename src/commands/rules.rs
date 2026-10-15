mod create;
mod definition;
mod delete;
mod get;
mod list;
#[cfg(test)]
mod tests;
mod update;

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, credentials::CredentialStore,
    error::AppError, input::ConfirmationProvider,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct RuleArgs {
    #[command(subcommand)]
    pub command: RuleCommand,
}
#[derive(Debug, Subcommand)]
pub enum RuleCommand {
    /// List a board's automation rules and embedded trigger/action fields.
    List(list::ListArgs),
    /// Show a board automation rule.
    Get(get::GetArgs),
    /// Create a rule from a typed camelCase JSON definition.
    Create(create::CreateArgs),
    /// Merge a sparse JSON definition into a rule.
    Update(update::UpdateArgs),
    /// Delete a rule and its trigger/action, with confirmation or --yes.
    Delete(delete::DeleteArgs),
}
pub(crate) async fn dispatch(
    command: RuleCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        RuleCommand::List(args) => list::execute(args, factory, store).await,
        RuleCommand::Get(args) => get::execute(args, factory, store).await,
        RuleCommand::Create(args) => create::execute(args, factory, store).await,
        RuleCommand::Update(args) => update::execute(args, factory, store).await,
        RuleCommand::Delete(args) => delete::execute(args, factory, store, confirmation).await,
    }
}
#[derive(Debug, Args)]
struct Target {
    /// Wekan rule ID.
    #[arg(value_parser = identifier, help_heading = "Target")]
    rule_id: String,
    /// Wekan board ID.
    #[arg(long = "board", value_parser = identifier, help_heading = "Target")]
    board_id: String,
}
fn identifier(value: &str) -> Result<String, String> {
    super::transfer::identifier(value)
}
