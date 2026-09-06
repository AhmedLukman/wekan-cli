use clap::Args;

use crate::{
    client::{SwimlaneDocument, WekanClientFactory},
    command_result::{CommandSuccess, SwimlaneDetail},
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
    #[arg(long = "board", value_parser = non_empty)]
    pub board_id: String,

    /// Wekan swimlane ID.
    #[arg(value_parser = non_empty)]
    pub swimlane_id: String,
}

pub(super) async fn execute(
    args: GetArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let swimlane = context
        .client()
        .swimlane(&args.board_id, &args.swimlane_id, context.record().token())
        .await
        .map_err(|error| {
            map_client_error_with_not_found(
                error,
                &redactor,
                "swimlane lookup",
                "the requested Wekan swimlane was not found on that board",
            )
        })?;
    Ok(CommandSuccess::SwimlaneShown(swimlane_detail(swimlane)))
}

fn swimlane_detail(swimlane: SwimlaneDocument) -> SwimlaneDetail {
    SwimlaneDetail {
        swimlane_id: swimlane.swimlane_id,
        title: swimlane.title,
        archived: swimlane.archived,
        archived_at: swimlane.archived_at,
        board_id: swimlane.board_id,
        created_at: swimlane.created_at,
        sort: swimlane.sort,
        color: swimlane.color,
        updated_at: swimlane.updated_at,
        modified_at: swimlane.modified_at,
        swimlane_type: swimlane.swimlane_type,
        height: swimlane.height,
    }
}
