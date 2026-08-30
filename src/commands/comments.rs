mod create;
mod delete;
mod get;
mod list;

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess, config::MissingProfileResolution,
    credentials::CredentialStore, error::AppError, input::ConfirmationProvider,
};

#[derive(Debug, Args)]
pub struct CommentArgs {
    #[command(subcommand)]
    pub command: CommentCommand,
}

#[derive(Debug, Subcommand)]
pub enum CommentCommand {
    /// List comments on a card.
    List(list::ListArgs),

    /// Show one card comment.
    Get(get::GetArgs),

    /// Create a comment on a card.
    Create(create::CreateArgs),

    /// Permanently delete a card comment.
    Delete(delete::DeleteArgs),
}

impl CommentCommand {
    pub(crate) const fn missing_profile_resolution(&self) -> MissingProfileResolution {
        MissingProfileResolution::Reject
    }
}

pub(crate) async fn dispatch(
    command: CommentCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        CommentCommand::List(args) => list::execute(args, client_factory, credential_store).await,
        CommentCommand::Get(args) => get::execute(args, client_factory, credential_store).await,
        CommentCommand::Create(args) => {
            create::execute(args, client_factory, credential_store).await
        }
        CommentCommand::Delete(args) => {
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
