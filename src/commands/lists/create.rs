use clap::Args;

use crate::{
    client::{CreateListRequest, WekanClientFactory},
    command_result::{CommandSuccess, ListCreateSuccess},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::{non_empty, trimmed_non_empty};

#[derive(Debug, Args)]
#[command(
    after_help = "Example:\n  wekan list create --board board-id --title \"Plan next release\""
)]
#[command(next_help_heading = "Fields")]
pub struct CreateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty, help_heading = "Target")]
    pub board_id: String,

    /// List title.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub title: String,

    /// Swimlane ID; defaults to the board's default swimlane.
    #[arg(long, value_parser = non_empty)]
    pub swimlane_id: Option<String>,
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
        .create_list(
            &args.board_id,
            &CreateListRequest {
                title: args.title,
                swimlane_id: args.swimlane_id,
            },
            context.record().token(),
        )
        .await
        .map_err(|error| map_client_error(error, &redactor, "list creation", true))?;
    Ok(CommandSuccess::ListCreated(ListCreateSuccess {
        board_id: args.board_id,
        list_id: created.list_id,
    }))
}
