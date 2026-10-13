mod create;
mod delete;
mod list;
#[cfg(test)]
mod tests;
mod update;

use crate::{
    client::{WekanClientFactory, dependencies::DependencyMetadata},
    command_result::{CommandSuccess, DependencySubmittedField},
    credentials::CredentialStore,
    error::AppError,
    input::ConfirmationProvider,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct DependencyArgs {
    #[command(subcommand)]
    pub command: DependencyCommand,
}

#[derive(Debug, Subcommand)]
pub enum DependencyCommand {
    /// List a board's dependency lines or one card's outgoing dependencies.
    List(list::ListArgs),
    /// Create or update a dependency to another card on the same board.
    Create(create::CreateArgs),
    /// Update dependency metadata; omitted fields stay unchanged.
    Update(update::UpdateArgs),
    /// Remove a dependency line, with confirmation or --yes.
    Delete(delete::DeleteArgs),
}

pub(crate) async fn dispatch(
    command: DependencyCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        DependencyCommand::List(args) => list::execute(args, factory, store).await,
        DependencyCommand::Create(args) => create::execute(args, factory, store).await,
        DependencyCommand::Update(args) => update::execute(args, factory, store).await,
        DependencyCommand::Delete(args) => {
            delete::execute(args, factory, store, confirmation).await
        }
    }
}

#[derive(Debug, Args)]
struct MutationTarget {
    /// Wekan target card ID; the dependency is stored on --card.
    #[arg(value_parser = identifier, help_heading = "Target")]
    target_card_id: String,
    /// Wekan board ID.
    #[arg(long = "board", value_parser = identifier, help_heading = "Target")]
    board_id: String,
    /// Wekan source card ID.
    #[arg(long = "card", value_parser = identifier, help_heading = "Target")]
    card_id: String,
}

#[derive(Debug, Args)]
struct MetadataArgs {
    /// Relation: related-to, blocks, is-blocked-by, fixes, or is-fixed-by; also accepts empty.
    #[arg(long = "type", value_name = "TYPE", value_parser = dependency_type, conflicts_with = "null_type", help_heading = "Metadata")]
    dependency_type: Option<String>,
    /// Send JSON null for the relation type.
    #[arg(long, help_heading = "Metadata")]
    null_type: bool,
    /// CSS color, sent unchanged; may be empty.
    #[arg(long, conflicts_with = "null_color", help_heading = "Metadata")]
    color: Option<String>,
    /// Send JSON null for the color.
    #[arg(long, help_heading = "Metadata")]
    null_color: bool,
    /// FontAwesome icon name without fa-, sent unchanged; may be empty.
    #[arg(long, conflicts_with = "null_icon", help_heading = "Metadata")]
    icon: Option<String>,
    /// Send JSON null for the icon.
    #[arg(long, help_heading = "Metadata")]
    null_icon: bool,
}

impl MetadataArgs {
    fn into_request(self) -> DependencyMetadata {
        DependencyMetadata {
            dependency_type: nullable(self.dependency_type, self.null_type),
            color: nullable(self.color, self.null_color),
            icon: nullable(self.icon, self.null_icon),
        }
    }
}

fn submitted_fields(request: &DependencyMetadata) -> Vec<DependencySubmittedField> {
    let mut fields = Vec::new();
    if request.dependency_type.is_some() {
        fields.push(DependencySubmittedField::Type);
    }
    if request.color.is_some() {
        fields.push(DependencySubmittedField::Color);
    }
    if request.icon.is_some() {
        fields.push(DependencySubmittedField::Icon);
    }
    fields
}

fn nullable(value: Option<String>, null: bool) -> Option<Option<String>> {
    if null { Some(None) } else { value.map(Some) }
}

fn identifier(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || matches!(value, "." | "..") {
        Err("ID must not be empty, whitespace, or dot-only".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

fn dependency_type(value: &str) -> Result<String, String> {
    if matches!(
        value,
        "" | "related-to" | "blocks" | "is-blocked-by" | "fixes" | "is-fixed-by"
    ) {
        Ok(value.to_owned())
    } else {
        Err("type must be related-to, blocks, is-blocked-by, fixes, is-fixed-by, or an empty string".to_owned())
    }
}
