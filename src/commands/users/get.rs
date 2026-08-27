use clap::Args;

use crate::{
    client::WekanClientFactory, command_result::CommandSuccess,
    commands::authenticated::AuthenticatedContext, credentials::CredentialStore, error::AppError,
    redaction::Redactor,
};

use super::{map_client_error, non_empty, user_detail};

#[derive(Debug, Args)]
pub struct GetArgs {
    /// Wekan user ID or username.
    #[arg(value_parser = non_empty)]
    pub user: String,
}

pub(super) async fn execute(
    args: GetArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let user = context
        .client()
        .user(&args.user, context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "user lookup", false))?;
    Ok(CommandSuccess::UserShown(user_detail(user)))
}
