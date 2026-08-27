use clap::Args;

use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    commands::authenticated::AuthenticatedContext,
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    redaction::Redactor,
};

use super::{map_client_error, user_detail};

#[derive(Debug, Args)]
pub struct CurrentArgs {}

pub(super) async fn execute(
    _args: CurrentArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let user = context
        .client()
        .current_user(context.record().token())
        .await
        .map_err(|error| map_client_error(error, &redactor, "current user", false))?;
    if user.user_id() != context.record().user_id() {
        return Err(AppError::new(
            ErrorCode::CredentialStoreFailed,
            "the stored credential user id did not match the authenticated Wekan user",
            StableExitCode::Credential,
        ));
    }
    Ok(CommandSuccess::UserCurrent(user_detail(user)))
}
