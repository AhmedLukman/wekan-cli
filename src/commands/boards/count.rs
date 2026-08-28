use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{BoardCountSuccess, CommandSuccess},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

#[derive(Debug, Args)]
pub struct CountArgs {}

pub(super) async fn execute(
    _args: CountArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let counts = context
        .client()
        .board_counts(context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "board count", false))?;
    Ok(CommandSuccess::BoardCount(BoardCountSuccess {
        private: counts.private,
        public: counts.public,
    }))
}
