mod attachments;
mod card_settings;
mod chart;
mod copy;
mod count;
mod create;
mod delete;
mod domains;
mod export;
mod export_all;
mod get;
mod import;
mod import_trello;
mod import_zip;
mod labels;
mod list;
mod members;
mod presentation;
mod rename;

#[cfg(test)]
mod tests;

use clap::{Args, Subcommand};

use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    input::ConfirmationProvider,
};

#[derive(Debug, Args)]
pub struct BoardArgs {
    #[command(subcommand)]
    pub command: BoardCommand,
}

#[derive(Debug, Subcommand)]
pub enum BoardCommand {
    /// Export several or all eligible boards in one archive or workbook.
    ExportAll(Box<export_all::ExportAllArgs>),
    /// Export native board report charts.
    Chart(chart::ChartArgs),
    /// Read or change board view visibility, defaults, and order.
    ViewSettings(presentation::ViewSettingsArgs),
    /// Read or change the order of sections on opened cards.
    CardFieldOrder(presentation::CardFieldOrderArgs),
    /// List attachment links using Wekan's legacy board projection.
    Attachment(attachments::AttachmentArgs),
    /// Import an export document into a new board.
    Import(import::ImportArgs),
    /// Import Trello JSON or ZIP into new boards using /import-trello.
    ImportTrello(import_trello::ImportTrelloArgs),
    /// Import a native ZIP archive into an existing board.
    ImportZip(import_zip::ImportZipArgs),
    /// Export a native artifact to a file or validated raw stdout.
    Export(Box<export::ExportArgs>),
    /// Read or update card display controls and aging thresholds.
    CardSettings(card_settings::CardSettingsArgs),
    /// List or request changes to email-domain sharing.
    Domain(domains::DomainArgs),
    /// Copy a board using Wekan's native copy operation.
    Copy(copy::CopyArgs),
    /// List or add reusable board labels.
    Label(labels::LabelArgs),
    /// List, add, remove, or change board members.
    Member(members::MemberArgs),
    /// List active boards for the authenticated user, or all public boards.
    List(list::ListArgs),

    /// Count all private and public boards on the server.
    Count(count::CountArgs),

    /// Show a board by ID.
    Get(get::GetArgs),

    /// Create a board and its default swimlane.
    Create(create::CreateArgs),

    /// Rename a board.
    Rename(rename::RenameArgs),

    /// Permanently delete a board and all of its contents.
    Delete(delete::DeleteArgs),
}

pub(crate) async fn dispatch(
    command: BoardCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    secrets: &dyn SecretInputProvider,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        BoardCommand::ExportAll(args) => {
            export_all::execute(*args, client_factory, credential_store).await
        }
        BoardCommand::Chart(args) => {
            chart::dispatch(args.command, client_factory, credential_store).await
        }
        BoardCommand::ViewSettings(args) => {
            presentation::views(args.command, client_factory, credential_store).await
        }
        BoardCommand::CardFieldOrder(args) => {
            presentation::order(args.command, client_factory, credential_store).await
        }
        BoardCommand::Attachment(args) => {
            attachments::dispatch(args.command, client_factory, credential_store).await
        }
        BoardCommand::Import(args) => {
            import::execute(args, client_factory, credential_store, secrets).await
        }
        BoardCommand::ImportTrello(args) => {
            import_trello::execute(args, client_factory, credential_store).await
        }
        BoardCommand::ImportZip(args) => {
            import_zip::execute(args, client_factory, credential_store).await
        }
        BoardCommand::Export(args) => {
            export::execute(*args, client_factory, credential_store).await
        }
        BoardCommand::CardSettings(args) => {
            card_settings::dispatch(args.command, client_factory, credential_store).await
        }
        BoardCommand::Domain(args) => {
            domains::dispatch(args.command, client_factory, credential_store).await
        }
        BoardCommand::Copy(args) => copy::execute(args, client_factory, credential_store).await,
        BoardCommand::Label(args) => {
            labels::dispatch(args.command, client_factory, credential_store).await
        }
        BoardCommand::Member(args) => {
            members::dispatch(args.command, client_factory, credential_store).await
        }
        BoardCommand::List(args) => list::execute(args, client_factory, credential_store).await,
        BoardCommand::Count(args) => count::execute(args, client_factory, credential_store).await,
        BoardCommand::Get(args) => get::execute(args, client_factory, credential_store).await,
        BoardCommand::Create(args) => create::execute(args, client_factory, credential_store).await,
        BoardCommand::Rename(args) => rename::execute(args, client_factory, credential_store).await,
        BoardCommand::Delete(args) => {
            delete::execute(args, client_factory, credential_store, confirmation).await
        }
    }
}

fn non_empty(value: &str) -> Result<String, String> {
    if value.is_empty() {
        Err("value must not be empty".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

fn trimmed_non_empty(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err("value must not be empty or whitespace".to_owned())
    } else {
        Ok(value.to_owned())
    }
}
