use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{CommandSuccess, ListCollectionSuccess, ListSummary},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    pub board_id: String,
}

pub(super) async fn execute(
    args: ListArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let lists = context
        .client()
        .board_lists(&args.board_id, context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "board list collection", false))?;
    Ok(CommandSuccess::ListCollection(ListCollectionSuccess {
        board_id: args.board_id,
        lists: lists
            .into_iter()
            .map(|list| ListSummary {
                list_id: list.list_id,
                title: list.title,
                modified_at: list.modified_at,
                cards_modified_at: list.cards_modified_at,
            })
            .collect(),
    }))
}
