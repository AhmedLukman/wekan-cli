use clap::Args;

use crate::{
    client::{CreateUserRequest, WekanClientFactory},
    command_result::{CommandSuccess, UserCreateSuccess, UserCreateWarning},
    commands::authenticated::AuthenticatedContext,
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    redaction::Redactor,
};

use super::{map_client_error, non_empty};

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Username for the new account.
    #[arg(long, value_parser = non_empty)]
    pub username: String,

    /// Email address for the new account.
    #[arg(long, value_parser = non_empty)]
    pub email: String,

    /// Read one password line from standard input instead of prompting twice.
    #[arg(long)]
    pub password_stdin: bool,
}

pub(super) async fn execute(
    args: CreateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let password = secret_input.read_new_account_password(args.password_stdin)?;
    let redactor = Redactor::with_secret(context.record().token()).and_secret(&password);
    context
        .client()
        .create_user(
            &CreateUserRequest {
                username: args.username.clone(),
                email: args.email.clone(),
                password: password.clone(),
            },
            context.record().token(),
        )
        .await
        .map_err(|error| map_client_error(error, &redactor, "user creation", true))?;
    Ok(CommandSuccess::UserCreated(UserCreateSuccess {
        created: true,
        username: args.username,
        email: args.email,
        user_id: None,
        warning: Some(UserCreateWarning::UserIdUnavailableInWekanV1106),
    }))
}
