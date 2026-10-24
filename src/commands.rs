pub mod admin;
pub mod api;
pub mod attachments;
pub mod auth;
pub(crate) mod authenticated;
pub mod boards;
pub mod cards;
pub mod checklist_items;
pub mod checklists;
pub(crate) mod client_error;
pub mod comments;
pub(crate) mod credential_ops;
pub mod custom_field_items;
pub mod custom_fields;
pub mod dependencies;
pub(crate) mod exports;
pub(crate) mod imports;
pub mod integrations;
pub mod lists;
pub mod profile;
pub mod rules;
pub mod skill;
pub mod swimlanes;
pub mod system;
mod transfer;
pub mod users;

#[cfg(test)]
mod tests;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum RootCommand {
    /// Read agent guidance embedded in this CLI version.
    Skill(skill::SkillArgs),
    /// Inspect public Wekan system status.
    System(system::SystemArgs),
    /// Report domains and configure global settings, organizations, and teams.
    Admin(Box<admin::AdminArgs>),
    /// Inspect and manage board automation rules.
    Rule(rules::RuleArgs),
    /// Inspect and manage board integrations and webhook subscriptions.
    Integration(integrations::IntegrationArgs),
    /// Inspect and manage card dependency lines.
    Dependency(dependencies::DependencyArgs),
    /// Inspect, transfer and manage Wekan attachments.
    Attachment(attachments::AttachmentArgs),
    /// Send a low-level request to the selected Wekan server.
    Api(api::ApiArgs),

    /// Authenticate with a Wekan server.
    Auth(auth::AuthArgs),

    /// Manage named local Wekan server profiles.
    Profile(profile::ProfileArgs),

    /// Inspect and administer Wekan users.
    User(users::UserArgs),

    /// Inspect and manage Wekan boards.
    Board(boards::BoardArgs),

    /// Inspect and manage lists on Wekan boards.
    List(lists::ListArgs),

    /// Inspect and manage cards in Wekan board lists.
    Card(Box<cards::CardArgs>),

    /// Inspect and manage checklists on Wekan cards.
    Checklist(checklists::ChecklistArgs),

    /// Inspect and manage comments on Wekan cards.
    Comment(comments::CommentArgs),

    /// Inspect and manage custom-field definitions.
    CustomField(custom_fields::CustomFieldArgs),

    /// Inspect and manage swimlanes on Wekan boards.
    Swimlane(swimlanes::SwimlaneArgs),
}

impl RootCommand {
    pub fn supports_raw_output(&self) -> bool {
        matches!(self, Self::Api(_))
            || self.export_delivery().is_some_and(|d| d.is_stdout())
            || matches!(self, Self::Attachment(args) if args.command.download_destination().is_some_and(|d| d.is_stdout()))
    }

    pub(crate) fn export_delivery(&self) -> Option<&exports::DeliveryArgs> {
        match self {
            Self::Board(a) => match &a.command {
                boards::BoardCommand::Export(e) => Some(&e.delivery),
                boards::BoardCommand::ExportAll(e) => Some(&e.delivery),
                boards::BoardCommand::Chart(a) => a.delivery(),
                _ => None,
            },
            Self::Card(a) => {
                if let cards::CardCommand::Export(e) = &a.command {
                    Some(&e.delivery)
                } else {
                    None
                }
            }
            Self::Attachment(a) => {
                if let attachments::AttachmentCommand::Export(e) = &a.command {
                    Some(&e.delivery)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    pub(crate) fn validate_input(&self) -> Result<(), crate::error::AppError> {
        if let Self::Board(a) = self {
            if let boards::BoardCommand::Import(args) = &a.command {
                args.validate()?;
            }
            if let boards::BoardCommand::ImportTrello(args) = &a.command {
                args.validate()?;
            }
        }
        if let Self::Board(a) = self {
            if let boards::BoardCommand::Export(e) = &a.command {
                e.validate()?;
            }
        }
        if let Some(delivery) = self.export_delivery() {
            delivery.validate()?;
        }
        Ok(())
    }
    pub fn validate_output(
        &self,
        format: crate::output::OutputFormat,
    ) -> Result<(), crate::error::AppError> {
        self.validate_input()?;
        if let Some(delivery) = self.export_delivery() {
            delivery.validate_output(format)?;
        }
        if let Self::Attachment(args) = self {
            args.command.validate_input()?;
            if let Some(destination) = args.command.download_destination() {
                destination.validate_output(format)?;
            }
        }
        if format == crate::output::OutputFormat::Raw && !self.supports_raw_output() {
            return Err(crate::error::AppError::invalid_input(
                "--output raw requires api request, an export, or an attachment download with --destination -",
            ));
        }
        Ok(())
    }
}
