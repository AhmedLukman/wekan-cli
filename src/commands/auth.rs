pub mod login;
pub mod register;
pub mod status;

use clap::{Args, Subcommand};
use time::format_description::well_known::Rfc3339;

use crate::{
    client::{AuthSession, WekanClientFactory},
    credentials::{CredentialRecord, CredentialStore, SecretInputProvider},
    error::{AppError, ErrorCode},
    exit_code::StableExitCode,
    output::{AuthSuccess, CommandSuccess},
    redaction::Redactor,
};

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Log in to Wekan and securely store the returned token.
    Login(login::LoginArgs),

    /// Register a Wekan account and securely store its login token.
    Register(register::RegisterArgs),

    /// Validate and display the stored authentication session.
    Status(status::StatusArgs),
}

pub(crate) async fn dispatch(
    command: AuthCommand,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    secret_input: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        AuthCommand::Login(args) => {
            login::execute(args, client_factory, credential_store, secret_input).await
        }
        AuthCommand::Register(args) => {
            register::execute(args, client_factory, credential_store, secret_input).await
        }
        AuthCommand::Status(args) => status::execute(args, client_factory, credential_store).await,
    }
}

pub(super) fn preflight_credentials(
    credential_store: &dyn CredentialStore,
    server_url: &str,
) -> Result<(), AppError> {
    credential_store
        .check_available(server_url)
        .map_err(|error| {
            AppError::new(
                ErrorCode::CredentialStoreUnavailable,
                error.to_string(),
                StableExitCode::Credential,
            )
        })
}

pub(super) fn persist_session(
    credential_store: &dyn CredentialStore,
    server_url: String,
    session: AuthSession,
) -> Result<AuthSuccess, AppError> {
    let (user_id, token, token_expires) = session.into_parts();
    let token_expires_text = token_expires.format(&Rfc3339).map_err(|_| {
        AppError::new(
            ErrorCode::InternalError,
            "the validated token expiry could not be formatted",
            StableExitCode::Internal,
        )
    })?;
    let record = CredentialRecord::new(server_url.clone(), user_id.clone(), token, token_expires);
    let token_redactor = Redactor::with_secret(record.token());

    credential_store
        .save(&server_url, &record)
        .map_err(|error| {
            AppError::new(
                ErrorCode::CredentialStoreFailed,
                token_redactor.redact(&error.to_string()),
                StableExitCode::Credential,
            )
        })?;

    Ok(AuthSuccess {
        server: server_url,
        user_id,
        token_expires: token_expires_text,
        credential_stored: true,
    })
}
