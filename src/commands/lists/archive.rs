use super::non_empty;
use crate::{
    client::WekanClientFactory,
    command_result::{CommandSuccess, ListArchiveSuccess},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
pub struct ArchiveArgs {
    /// Wekan list ID.
    #[arg(value_parser = non_empty)]
    list_id: String,
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    board_id: String,
}
pub(super) async fn execute(
    args: ArchiveArgs,
    archived: bool,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let response = context
        .client()
        .set_list_archived(
            &args.board_id,
            &args.list_id,
            archived,
            context.record().token(),
        )
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "list archive/restore",
                "the requested Wekan list was not found in the required archived state",
            )
        })?;
    let result = ListArchiveSuccess {
        board_id: args.board_id,
        list_id: response.list_id,
        archived: response.archived,
    };
    Ok(if archived {
        CommandSuccess::ListArchived(result)
    } else {
        CommandSuccess::ListRestored(result)
    })
}
