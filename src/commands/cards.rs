mod archive;
mod bulk;
mod copy;
mod count;
mod create;
mod delete;
mod find_by_custom_field;
mod get;
mod labels;
mod list;
mod membership;
mod restore;
mod set_custom_field;
mod update;

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};

use crate::{
    client::WekanClientFactory,
    command_result::{CardMembershipKind, CommandSuccess},
    credentials::CredentialStore,
    error::AppError,
    input::ConfirmationProvider,
};

#[derive(Debug, Args)]
pub struct CardArgs {
    #[command(subcommand)]
    pub command: CardCommand,
}

#[derive(Debug, Subcommand)]
pub enum CardCommand {
    /// Count active cards on a board or in a list.
    Count(count::CountArgs),
    /// Copy a card using Wekan's native copy operation.
    Copy(copy::CopyArgs),
    /// Create, delete, or merge labels on up to 500 cards per request.
    Bulk(bulk::BulkArgs),
    /// List the label IDs stored on a card.
    Label(labels::LabelArgs),
    /// List or merge individual card members.
    Member(membership::MembershipArgs),
    /// List or merge individual card assignees.
    Assignee(membership::MembershipArgs),
    /// List active cards in a board list or swimlane.
    List(list::ListArgs),

    /// Show a card by ID, optionally restricted to a board and list.
    Get(get::GetArgs),

    /// Assign an existing custom-field entry on an active card.
    SetCustomField(set_custom_field::SetCustomFieldArgs),
    /// Find active cards using Wekan's string custom-field query.
    FindByCustomField(find_by_custom_field::FindByCustomFieldArgs),
    /// Create a card in a board list.
    Create(Box<create::CreateArgs>),

    /// Update one or more editable card fields.
    Update(Box<update::UpdateArgs>),

    /// Archive an active card and its child cards.
    Archive(archive::ArchiveArgs),

    /// Restore an archived card and its child cards.
    Restore(restore::RestoreArgs),

    /// Permanently delete a card and its child resources.
    Delete(delete::DeleteArgs),
}

pub(crate) async fn dispatch(
    command: CardCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        CardCommand::Count(args) => count::execute(args, client_factory, credential_store).await,
        CardCommand::Copy(args) => copy::execute(args, client_factory, credential_store).await,
        CardCommand::Bulk(args) => {
            bulk::dispatch(args.command, client_factory, credential_store, confirmation).await
        }
        CardCommand::Label(args) => {
            labels::dispatch(args.command, client_factory, credential_store).await
        }
        CardCommand::Member(args) => {
            membership::dispatch(
                args.command,
                CardMembershipKind::Member,
                client_factory,
                credential_store,
            )
            .await
        }
        CardCommand::Assignee(args) => {
            membership::dispatch(
                args.command,
                CardMembershipKind::Assignee,
                client_factory,
                credential_store,
            )
            .await
        }
        CardCommand::List(args) => list::execute(args, client_factory, credential_store).await,
        CardCommand::Get(args) => get::execute(args, client_factory, credential_store).await,
        CardCommand::SetCustomField(args) => {
            set_custom_field::execute(args, client_factory, credential_store).await
        }
        CardCommand::FindByCustomField(args) => {
            find_by_custom_field::execute(args, client_factory, credential_store).await
        }
        CardCommand::Create(args) => create::execute(*args, client_factory, credential_store).await,
        CardCommand::Update(args) => update::execute(*args, client_factory, credential_store).await,
        CardCommand::Archive(args) => {
            archive::execute(args, client_factory, credential_store).await
        }
        CardCommand::Restore(args) => {
            restore::execute(args, client_factory, credential_store).await
        }
        CardCommand::Delete(args) => {
            delete::execute(args, client_factory, credential_store, confirmation).await
        }
    }
}

fn non_empty(value: &str) -> Result<String, String> {
    trimmed_non_empty(value)
}

fn trimmed_non_empty(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err("value must not be empty or whitespace".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

fn date_input(value: &str) -> Result<crate::client::cards::CardDateInput, String> {
    use crate::client::cards::CardDateInput;
    Ok(match value.trim().parse::<serde_json::Number>() {
        Ok(number) => CardDateInput::EpochMilliseconds(number),
        Err(_) => CardDateInput::Text(value.to_owned()),
    })
}
