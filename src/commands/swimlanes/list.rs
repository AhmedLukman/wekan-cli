use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{CommandSuccess, SwimlaneCollectionSuccess, SwimlaneSummary},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::non_empty;

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Wekan board ID.
    #[arg(value_parser = non_empty)]
    pub board_id: String,
}

pub(super) async fn execute(
    args: ListArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let swimlanes = context
        .client()
        .board_swimlanes(&args.board_id, context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "board swimlane collection", false))?;
    Ok(CommandSuccess::SwimlaneCollection(
        SwimlaneCollectionSuccess {
            board_id: args.board_id,
            swimlanes: swimlanes
                .into_iter()
                .map(|swimlane| SwimlaneSummary {
                    swimlane_id: swimlane.swimlane_id,
                    title: swimlane.title,
                })
                .collect(),
        },
    ))
}
