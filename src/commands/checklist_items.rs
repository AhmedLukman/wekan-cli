mod create;
mod delete;
mod get;
mod update;

use clap::{Args, Subcommand};

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, credentials::CredentialStore,
    error::AppError, input::ConfirmationProvider,
};

#[derive(Debug, Args)]
pub struct ChecklistItemArgs {
    #[command(subcommand)]
    pub command: ChecklistItemCommand,
}

#[derive(Debug, Subcommand)]
pub enum ChecklistItemCommand {
    /// Show one checklist item.
    Get(get::GetArgs),

    /// Create a checklist item on a card.
    Create(create::CreateArgs),

    /// Update the title or completion state of a checklist item.
    Update(update::UpdateArgs),

    /// Permanently delete a checklist item.
    Delete(delete::DeleteArgs),
}

pub(crate) async fn dispatch(
    command: ChecklistItemCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        ChecklistItemCommand::Update(args) => {
            update::execute(args, client_factory, credential_store).await
        }
        ChecklistItemCommand::Get(args) => {
            get::execute(args, client_factory, credential_store).await
        }
        ChecklistItemCommand::Create(args) => {
            create::execute(args, client_factory, credential_store).await
        }
        ChecklistItemCommand::Delete(args) => {
            delete::execute(args, client_factory, credential_store, confirmation).await
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

fn deadline(value: &str) -> Result<String, String> {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .map(|_| value.to_owned())
        .map_err(|_| "expected an RFC 3339 date-time with a timezone".to_owned())
}
