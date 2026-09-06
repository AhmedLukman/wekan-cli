pub mod api;
pub mod auth;
pub(crate) mod authenticated;
pub mod boards;
pub mod cards;
pub(crate) mod client_error;
pub mod comments;
pub(crate) mod credential_ops;
pub mod lists;
pub mod profile;
pub mod swimlanes;
pub mod users;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum RootCommand {
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

    /// Inspect and manage comments on Wekan cards.
    Comment(comments::CommentArgs),

    /// Inspect and manage swimlanes on Wekan boards.
    Swimlane(swimlanes::SwimlaneArgs),
}

impl RootCommand {
    pub const fn supports_raw_output(&self) -> bool {
        matches!(self, Self::Api(_))
    }
}
