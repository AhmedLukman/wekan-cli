use clap::Args;

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess,
    commands::authenticated::AuthenticatedContext, credentials::CredentialStore, error::AppError,
    redaction::Redactor,
};

use super::{map_client_error, non_empty, user_detail};

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Wekan user ID. With no action, Wekan returns the unchanged user.
    #[arg(value_parser = non_empty)]
    pub user_id: String,
}

pub(super) async fn execute(
    args: UpdateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let user = context
        .client()
        .update_user_without_action(&args.user_id, context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "user update", true))?;
    Ok(CommandSuccess::UserUpdated(user_detail(user)))
}
