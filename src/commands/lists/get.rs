use clap::Args;

use crate::{
    client::{ListDocument, WekanClientFactory},
    command_result::{CommandSuccess, ListDetail, ListWipLimitDetail},
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
pub struct GetArgs {
    /// Wekan board ID.
    #[arg(value_parser = non_empty)]
    pub board_id: String,

    /// Wekan list ID.
    #[arg(value_parser = non_empty)]
    pub list_id: String,
}

pub(super) async fn execute(
    args: GetArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let list = context
        .client()
        .list(&args.board_id, &args.list_id, context.record().token())
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "list lookup",
                "the requested Wekan list was not found on that board",
            )
        })?;
    Ok(CommandSuccess::ListShown(list_detail(list)))
}

fn list_detail(list: ListDocument) -> ListDetail {
    ListDetail {
        list_id: list.list_id,
        title: list.title,
        starred: list.starred,
        archived: list.archived,
        archived_at: list.archived_at,
        deleted_at: list.deleted_at,
        deleted_by: list.deleted_by,
        delete_batch_id: list.delete_batch_id,
        board_id: list.board_id,
        swimlane_id: list.swimlane_id,
        created_at: list.created_at,
        sort: list.sort,
        updated_at: list.updated_at,
        modified_at: list.modified_at,
        position_updated_at: list.position_updated_at,
        wip_limit: list.wip_limit.map(|limit| ListWipLimitDetail {
            value: limit.value,
            enabled: limit.enabled,
            soft: limit.soft,
        }),
        color: list.color,
        list_type: list.list_type,
        width: list.width,
    }
}
