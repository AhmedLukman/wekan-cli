use clap::Args;
use serde_json::Number;

use crate::{
    client::{CreateSwimlaneRequest, WekanClientFactory},
    command_result::{CommandSuccess, SwimlaneCreateSuccess},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::{finite_number, non_empty, trimmed_non_empty};

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Wekan board ID.
    #[arg(long = "board", value_parser = non_empty)]
    pub board_id: String,

    /// Swimlane title.
    #[arg(long, value_parser = trimmed_non_empty)]
    pub title: String,

    /// Explicit numeric sort value; omission appends after existing swimlanes.
    #[arg(long, value_parser = finite_number, allow_hyphen_values = true)]
    pub sort: Option<f64>,
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
        .create_swimlane(
            &args.board_id,
            &CreateSwimlaneRequest {
                title: args.title,
                sort: args.sort.map(|sort| {
                    Number::from_f64(sort).expect("the parser rejects non-finite numbers")
                }),
            },
            context.record().token(),
        )
        .await
        .map_err(|error| map_client_error(error, &redactor, "swimlane creation", true))?;
    Ok(CommandSuccess::SwimlaneCreated(SwimlaneCreateSuccess {
        board_id: args.board_id,
        swimlane_id: created.swimlane_id,
    }))
}
