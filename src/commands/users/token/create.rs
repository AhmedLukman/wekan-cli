use crate::{
    client::WekanClientFactory,
    command_result::{CommandSuccess, UserTokenCreateSuccess},
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::Args;

#[derive(Debug, Args)]
#[command(
    after_help = "The returned secret does not confirm that a session was stored. Wekan v11.06 does not await token insertion and returns an unusable token even for a nonexistent user."
)]
pub struct CreateArgs {
    /// Wekan user ID for which to issue a token.
    #[arg(value_parser = super::super::non_empty)]
    user_id: String,
}

pub(super) async fn execute(
    args: CreateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let response = context
        .client()
        .create_user_token(&args.user_id, context.record().token())
        .await
        .map_err(|e| {
            map_client_error(
                e,
                &Redactor::with_secret(context.record().token()),
                "token creation",
                true,
            )
        })?;
    Ok(CommandSuccess::UserTokenCreated(
        UserTokenCreateSuccess::new(response.user_id, response.auth_token),
    ))
}
