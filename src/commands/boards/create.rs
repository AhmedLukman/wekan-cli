use clap::{Args, ValueEnum};

use crate::{
    client::{BoardColor, BoardPermission, CreateBoardRequest, WekanClientFactory},
    command_result::{BoardCreateSuccess, CommandSuccess},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::trimmed_non_empty;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "lower")]
pub(super) enum BoardPermissionArg {
    Private,
    Public,
}

impl From<BoardPermissionArg> for BoardPermission {
    fn from(value: BoardPermissionArg) -> Self {
        match value {
            BoardPermissionArg::Private => Self::Private,
            BoardPermissionArg::Public => Self::Public,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "lower")]
pub(super) enum BoardColorArg {
    Belize,
    Nephritis,
    Pomegranate,
    Pumpkin,
    Wisteria,
    Moderatepink,
    Strongcyan,
    Limegreen,
    Midnight,
    Dark,
    Relax,
    Corteza,
    Appleglasspastel,
    Clearblue,
    Cleargreen,
    Clearorange,
    Clearpink,
    Clearpurple,
    Clearred,
    Natural,
    Modern,
    Moderndark,
    Exodark,
    Cleandark,
    Cleanlight,
}

impl From<BoardColorArg> for BoardColor {
    fn from(value: BoardColorArg) -> Self {
        match value {
            BoardColorArg::Belize => Self::Belize,
            BoardColorArg::Nephritis => Self::Nephritis,
            BoardColorArg::Pomegranate => Self::Pomegranate,
            BoardColorArg::Pumpkin => Self::Pumpkin,
            BoardColorArg::Wisteria => Self::Wisteria,
            BoardColorArg::Moderatepink => Self::Moderatepink,
            BoardColorArg::Strongcyan => Self::Strongcyan,
            BoardColorArg::Limegreen => Self::Limegreen,
            BoardColorArg::Midnight => Self::Midnight,
            BoardColorArg::Dark => Self::Dark,
            BoardColorArg::Relax => Self::Relax,
            BoardColorArg::Corteza => Self::Corteza,
            BoardColorArg::Appleglasspastel => Self::Appleglasspastel,
            BoardColorArg::Clearblue => Self::Clearblue,
            BoardColorArg::Cleargreen => Self::Cleargreen,
            BoardColorArg::Clearorange => Self::Clearorange,
            BoardColorArg::Clearpink => Self::Clearpink,
            BoardColorArg::Clearpurple => Self::Clearpurple,
            BoardColorArg::Clearred => Self::Clearred,
            BoardColorArg::Natural => Self::Natural,
            BoardColorArg::Modern => Self::Modern,
            BoardColorArg::Moderndark => Self::Moderndark,
            BoardColorArg::Exodark => Self::Exodark,
            BoardColorArg::Cleandark => Self::Cleandark,
            BoardColorArg::Cleanlight => Self::Cleanlight,
        }
    }
}

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Board title.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub title: String,

    /// User ID for the initial active administrator; defaults to the caller.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub owner: Option<String>,

    /// Board visibility.
    #[arg(long, value_enum, default_value = "private")]
    pub(super) permission: BoardPermissionArg,

    /// Board theme color.
    #[arg(long, value_enum, default_value = "belize")]
    pub(super) color: BoardColorArg,

    /// Set the initial member's no-comments flag.
    #[arg(long)]
    pub no_comments: bool,

    /// Set the initial member's comment-only flag.
    #[arg(long)]
    pub comment_only: bool,

    /// Set the initial member's worker flag.
    #[arg(long)]
    pub worker: bool,
}

pub(super) async fn execute(
    args: CreateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let created = context
        .client()
        .create_board(
            &CreateBoardRequest {
                title: args.title,
                owner: args.owner,
                permission: args.permission.into(),
                color: args.color.into(),
                is_no_comments: args.no_comments,
                is_comment_only: args.comment_only,
                is_worker: args.worker,
            },
            context.record().token(),
        )
        .await
        .map_err(|error| map_client_error(error, &redactor, "board creation", true))?;
    Ok(CommandSuccess::BoardCreated(BoardCreateSuccess {
        board_id: created.board_id,
        default_swimlane_id: created.default_swimlane_id,
    }))
}
