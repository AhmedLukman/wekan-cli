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
pub mod lists;
pub mod profile;
pub mod swimlanes;
pub mod users;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum RootCommand {
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

    /// Inspect and manage checklist items.
    ChecklistItem(checklist_items::ChecklistItemArgs),

    /// Inspect and manage comments on Wekan cards.
    Comment(comments::CommentArgs),

    /// Inspect and manage custom-field definitions.
    CustomField(custom_fields::CustomFieldArgs),

    /// Manage custom-field dropdown items.
    CustomFieldItem(custom_field_items::CustomFieldItemArgs),

    /// Inspect and manage swimlanes on Wekan boards.
    Swimlane(swimlanes::SwimlaneArgs),
}

impl RootCommand {
    pub fn supports_raw_output(&self) -> bool {
        matches!(self, Self::Api(_))
            || matches!(self, Self::Attachment(args) if args.command.download_destination().is_some_and(|d| d.is_stdout()))
    }

    pub fn validate_output(
        &self,
        format: crate::output::OutputFormat,
    ) -> Result<(), crate::error::AppError> {
        if let Self::Attachment(args) = self {
            args.command.validate_input()?;
            if let Some(destination) = args.command.download_destination() {
                destination.validate_output(format)?;
            }
        }
        if format == crate::output::OutputFormat::Raw && !self.supports_raw_output() {
            return Err(crate::error::AppError::invalid_input(
                "--output raw requires api request or an attachment download with --destination -",
            ));
        }
        Ok(())
    }
}
