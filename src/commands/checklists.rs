mod create;
mod delete;
mod get;
mod list;
mod update;

use clap::{Args, Subcommand};

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, credentials::CredentialStore,
    error::AppError, input::ConfirmationProvider,
};

#[derive(Debug, Args)]
pub struct ChecklistArgs {
    #[command(subcommand)]
    pub command: ChecklistCommand,
}

#[derive(Debug, Subcommand)]
pub enum ChecklistCommand {
    /// List checklists on a card.
    List(list::ListArgs),

    /// Show one card checklist.
    Get(get::GetArgs),

    /// Create a checklist on a card.
    Create(create::CreateArgs),

    /// Rename a checklist or set or clear its deadline.
    Update(update::UpdateArgs),

    /// Permanently delete a card checklist.
    Delete(delete::DeleteArgs),

    /// Inspect and manage checklist items.
    Item(super::checklist_items::ChecklistItemArgs),
}

pub(crate) async fn dispatch(
    command: ChecklistCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        ChecklistCommand::Update(args) => {
            update::execute(args, client_factory, credential_store).await
        }
        ChecklistCommand::List(args) => list::execute(args, client_factory, credential_store).await,
        ChecklistCommand::Get(args) => get::execute(args, client_factory, credential_store).await,
        ChecklistCommand::Create(args) => {
            create::execute(args, client_factory, credential_store).await
        }
        ChecklistCommand::Delete(args) => {
            delete::execute(args, client_factory, credential_store, confirmation).await
        }
        ChecklistCommand::Item(args) => {
            super::checklist_items::dispatch(
                args.command,
                client_factory,
                credential_store,
                confirmation,
            )
            .await
        }
    }
}

fn non_empty(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err("value must not be empty or whitespace".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

#[cfg(test)]
mod tests;
