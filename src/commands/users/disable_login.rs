use clap::Args;

use crate::{
    client::{UserAction, UserActionResult, WekanClientFactory},
    command_result::{
        CancellationSuccess, CommandSuccess, DestructiveOperation, UserLoginAction,
        UserLoginChangeSuccess,
    },
    commands::authenticated::AuthenticatedContext,
    credentials::CredentialStore,
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        confirm_or_skip,
    },
    redaction::Redactor,
};

use super::{map_client_error, non_empty, user_detail};

#[derive(Debug, Args)]
pub struct DisableLoginArgs {
    /// Wekan user ID whose login will be disabled.
    #[arg(value_parser = non_empty)]
    pub user_id: String,

    #[command(flatten)]
    pub confirmation: ConfirmationArgs,
}

pub(super) async fn execute(
    args: DisableLoginArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    if args.user_id == context.record().user_id() {
        return Err(AppError::invalid_input(
            "cannot disable login for the currently authenticated user",
        ));
    }
    let request = ConfirmationRequest::new(
        DestructiveOperation::UserDisableLogin,
        format!(
            "Disable login and clear all sessions for Wekan user `{}`?",
            args.user_id
        ),
    );
    if confirm_or_skip(&args.confirmation, confirmation, &request)?
        == ConfirmationDecision::Cancelled
    {
        return Ok(CommandSuccess::Cancelled(CancellationSuccess::new(
            DestructiveOperation::UserDisableLogin,
        )));
    }

    let redactor = Redactor::with_secret(context.record().token());
    let result = context
        .client()
        .user_action(
            &args.user_id,
            UserAction::DisableLogin,
            context.record().token(),
        )
        .await
        .map_err(|error| map_client_error(error, &redactor, "disable-login", true))?;
    let UserActionResult::LoginChanged(user) = result else {
        return Err(AppError::new(
            ErrorCode::ProtocolError,
            "Wekan returned the wrong response shape for the login action",
            StableExitCode::Server,
        ));
    };
    Ok(CommandSuccess::UserLoginChanged(UserLoginChangeSuccess {
        action: UserLoginAction::Disabled,
        user: user_detail(*user),
    }))
}
