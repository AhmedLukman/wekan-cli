use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::{CommandSuccess, UserListSuccess, UserSummary},
    commands::authenticated::AuthenticatedContext,
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};

use super::map_client_error;

#[derive(Debug, Args)]
pub struct ListArgs {}

pub(super) async fn execute(
    _args: ListArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let users = context
        .client()
        .users(context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "user list", false))?
        .into_iter()
        .map(|user| UserSummary {
            user_id: user.user_id,
            username: user.username,
        })
        .collect();
    Ok(CommandSuccess::UserList(UserListSuccess { users }))
}
