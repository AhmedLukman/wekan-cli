use clap::Args;

use crate::{
    client::{UserAction, UserActionResult, WekanClientFactory},
    command_result::{CommandSuccess, UserLoginAction, UserLoginChangeSuccess},
    commands::authenticated::AuthenticatedContext,
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    redaction::Redactor,
};

use super::{map_client_error, non_empty, user_detail};

#[derive(Debug, Args)]
pub struct EnableLoginArgs {
    /// Wekan user ID whose login will be enabled.
    #[arg(value_parser = non_empty)]
    pub user_id: String,
}

pub(super) async fn execute(
    args: EnableLoginArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let redactor = Redactor::with_secret(context.record().token());
    let result = context
        .client()
        .user_action(
            &args.user_id,
            UserAction::EnableLogin,
            context.record().token(),
        )
        .await
        .map_err(|error| map_client_error(error, &redactor, "enable-login", true))?;
    let UserActionResult::LoginChanged(user) = result else {
        return Err(AppError::new(
            ErrorCode::ProtocolError,
            "Wekan returned the wrong response shape for the login action",
            StableExitCode::Server,
        ));
    };
    Ok(CommandSuccess::UserLoginChanged(UserLoginChangeSuccess {
        action: UserLoginAction::Enabled,
        user: user_detail(*user),
    }))
}
