use clap::Args;

use crate::{
    client::{
        CreateUserRequest, WekanClientFactory,
        users::{PasswordDigestError, UserPassword},
    },
    command_result::{CommandSuccess, UserCreateSuccess, UserCreateWarning},
    commands::authenticated::AuthenticatedContext,
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    redaction::Redactor,
};

use super::map_client_error;

#[derive(Debug, Args)]
pub struct CreateArgs {
    /// Username for the new account.
    #[arg(long, default_value = "")]
    pub username: String,

    /// Email address for the new account.
    #[arg(long, default_value = "")]
    pub email: String,

    /// Read one password line from standard input instead of prompting twice.
    #[arg(long)]
    pub password_stdin: bool,

    /// Read an exact plaintext password through EOF, preserving line endings.
    #[arg(long, conflicts_with_all = ["password_stdin", "password_digest_stdin"])]
    pub password_raw_stdin: bool,

    /// Read {digest, algorithm: "sha-256"} JSON securely through EOF.
    #[arg(long, conflicts_with_all = ["password_stdin", "password_raw_stdin"])]
    pub password_digest_stdin: bool,
}

pub(super) async fn execute(
    args: CreateArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    if args.username.is_empty() && args.email.is_empty() {
        return Err(AppError::invalid_input(
            "provide a nonempty --username or --email; no request was sent",
        ));
    }
    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    let password = if args.password_digest_stdin {
        UserPassword::from_digest_json(&secret_input.read_raw_secret()?).map_err(|error| {
            AppError::invalid_input(match error {
                PasswordDigestError::InvalidObject => {
                    "expected a SHA-256 password digest object on stdin"
                }
                PasswordDigestError::InvalidDigest => {
                    "expected a 64-character digest with algorithm sha-256"
                }
            })
        })?
    } else if args.password_raw_stdin {
        UserPassword::Plaintext(secret_input.read_raw_secret()?)
    } else {
        UserPassword::Plaintext(secret_input.read_new_account_password(args.password_stdin)?)
    };
    let redactor = Redactor::with_secret(context.record().token()).and_secret(password.secret());
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
        creation_requested: true,
        mutation_confirmed: false,
        username: args.username,
        email: args.email,
        user_id: None,
        warning: Some(UserCreateWarning::UserIdUnavailableInWekanV1106),
    }))
}
