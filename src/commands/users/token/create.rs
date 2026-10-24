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
pub struct CreateArgs {
    /// Wekan user ID for which to issue a token.
    #[arg(value_parser = super::super::non_empty)]
    user_id: String,
    /// Required administrator audit reason for issuing this user's token.
    #[arg(long, value_parser = reason)]
    reason: String,
}

pub(super) async fn execute(
    args: CreateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let response = context
        .client()
        .create_user_token(&args.user_id, &args.reason, context.record().token())
        .await
        .map_err(|e| {
            map_client_error(
                e,
                &Redactor::with_secret(context.record().token()),
                "token creation",
            )
        })?;
    Ok(CommandSuccess::UserTokenCreated(
        UserTokenCreateSuccess::new(response.user_id, response.auth_token),
    ))
}

fn reason(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err("a nonempty token issuance reason is required".into())
    } else {
        Ok(value.into())
    }
}
