use crate::{
    client::{
        WekanClientFactory,
        admin::{
            PreparedSettingsUpdate, StorageSecret,
            login_providers::{OAUTH_PROVIDER_KEYS, UpdateOauthProviderRequest},
        },
    },
    command_result::CommandSuccess,
    commands::{authenticated::AuthenticatedContext, client_error::map_client_error},
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    redaction::Redactor,
};
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct OauthProviderArgs {
    #[command(subcommand)]
    pub command: OauthProviderCommand,
}
#[derive(Debug, Subcommand)]
pub enum OauthProviderCommand {
    /// Read provider configuration sources and secret-presence flags.
    Get,
    /// Save a provider's enabled state and client ID plus optional controls.
    Update(UpdateArgs),
}
#[derive(Debug, Args)]
#[command(
    after_help = "--enabled and --id are required because Wekan replaces both on every update. An omitted or empty secret preserves the existing secret."
)]
pub struct UpdateArgs {
    /// OAuth provider key.
    #[arg(value_parser=OAUTH_PROVIDER_KEYS)]
    provider: String,
    /// Enable or disable this provider.
    #[arg(long,action=clap::ArgAction::Set,required=true)]
    enabled: bool,
    /// Client ID; an explicit empty value clears the ID.
    #[arg(long)]
    id: String,
    /// Read the provider secret from a regular UTF-8 file.
    #[arg(long, value_name = "PATH", conflicts_with = "secret_stdin")]
    secret_file: Option<std::path::PathBuf>,
    /// Read the provider secret securely from UTF-8 stdin.
    #[arg(long)]
    secret_stdin: bool,
    /// Provider-specific login style; an empty value clears the override.
    #[arg(long,value_parser=["popup","redirect",""])]
    login_style: Option<String>,
    /// Shared login style; an empty value clears the override.
    #[arg(long,value_parser=["popup","redirect",""])]
    global_login_style: Option<String>,
    /// Allow or disallow merging existing users.
    #[arg(long)]
    merge_existing_users: Option<bool>,
}
pub(crate) async fn oauth(
    command: OauthProviderCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    secrets: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let submitted_secret = match &command {
        OauthProviderCommand::Update(args) => {
            super::input::secret(args.secret_file.clone(), args.secret_stdin, secrets).await?
        }
        OauthProviderCommand::Get => None,
    };
    let mut redactor = Redactor::with_secret(context.record().token());
    if let Some(secret) = &submitted_secret {
        redactor = redactor.and_secret(secret);
    }
    let response = match command {
        OauthProviderCommand::Get => {
            context
                .client()
                .oauth_provider_settings(context.record().token())
                .await
        }
        OauthProviderCommand::Update(args) => {
            let request = UpdateOauthProviderRequest {
                enabled: args.enabled,
                id: args.id,
                secret: submitted_secret.as_ref().cloned().map(StorageSecret),
                login_style: args.login_style,
                global_login_style: args.global_login_style,
                merge_existing_users: args.merge_existing_users,
            };
            let request = PreparedSettingsUpdate::new(&request).map_err(|_| {
                AppError::invalid_input(
                    "OAuth provider settings exceed the input limit; no request was sent",
                )
            })?;
            context
                .client()
                .update_oauth_provider(&args.provider, request, context.record().token())
                .await
        }
    }
    .map_err(|e| map_client_error(e, &redactor, "OAuth provider settings"))?;
    Ok(CommandSuccess::OauthProviderSettings(Box::new(
        response.into(),
    )))
}
#[derive(Debug, Args)]
pub struct PasswordlessArgs {
    #[command(subcommand)]
    pub command: PasswordlessCommand,
}
#[derive(Debug, Subcommand)]
pub enum PasswordlessCommand {
    /// Enable or disable email-code sign-in.
    Update(PasswordlessUpdateArgs),
}
#[derive(Debug, Args)]
pub struct PasswordlessUpdateArgs {
    /// Whether email-code sign-in is offered.
    #[arg(long,action=clap::ArgAction::Set,required=true)]
    enabled: bool,
}
pub(crate) async fn passwordless(
    command: PasswordlessCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    let PasswordlessCommand::Update(args) = command;
    let context = AuthenticatedContext::load(factory, store)?;
    let response = context
        .client()
        .update_passwordless_settings(args.enabled, context.record().token())
        .await
        .map_err(|e| {
            map_client_error(
                e,
                &Redactor::with_secret(context.record().token()),
                "passwordless settings update",
            )
        })?;
    Ok(CommandSuccess::PasswordlessSettings(response.into()))
}
