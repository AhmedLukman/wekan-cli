use crate::{
    client::{WekanClientFactory, integrations::UpdateIntegrationRequest},
    command_result::{
        CommandSuccess, IntegrationSubmittedField as Field, IntegrationUpdateSuccess,
    },
    commands::{
        authenticated::AuthenticatedContext, client_error::map_client_error_with_not_found,
    },
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    redaction::Redactor,
};
use clap::Args;
use secrecy::{ExposeSecret, SecretString};

#[derive(Debug, Args)]
#[command(
    after_help = "Activity flags replace the complete subscription array. Empty updates send {}. Wekan writes fields sequentially and can retain earlier writes after a later failure."
)]
pub struct UpdateArgs {
    #[command(flatten)]
    target: super::Target,
    /// Explicit enabled state: true or false.
    #[arg(long)]
    enabled: Option<bool>,
    /// New title, sent unchanged; an empty string clears it.
    #[arg(long)]
    title: Option<String>,
    /// New webhook URL, sent unchanged for Wekan to validate.
    #[arg(long)]
    url: Option<String>,
    /// Read a token from UTF-8 stdin, removing one final LF or CRLF.
    #[arg(long, conflicts_with = "clear_token")]
    token_stdin: bool,
    /// Send an empty token string to clear the optional token.
    #[arg(long)]
    clear_token: bool,
    /// Replace subscriptions; repeat for each activity, including all.
    #[arg(long = "activity", conflicts_with = "clear_activities")]
    activities: Vec<String>,
    /// Replace the subscription array with an empty array.
    #[arg(long)]
    clear_activities: bool,
}

pub(super) async fn execute(
    args: UpdateArgs,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    secrets: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    let context = AuthenticatedContext::load(factory, store)?;
    let token = if args.token_stdin {
        let raw = secrets.read_raw_secret()?;
        let value = raw.expose_secret();
        let value = value
            .strip_suffix("\r\n")
            .or_else(|| value.strip_suffix('\n'))
            .unwrap_or(value);
        if value.is_empty() {
            return Err(AppError::invalid_input(
                "token stdin must not be empty; use --clear-token to clear the token; no request was sent",
            ));
        }
        Some(SecretString::from(value.to_owned()))
    } else if args.clear_token {
        Some(SecretString::from(String::new()))
    } else {
        None
    };
    let request = UpdateIntegrationRequest {
        enabled: args.enabled,
        title: args.title,
        url: args.url,
        token,
        activities: if args.clear_activities {
            Some(Vec::new())
        } else if args.activities.is_empty() {
            None
        } else {
            Some(args.activities)
        },
    };
    let mut fields = Vec::new();
    for (present, field) in [
        (request.enabled.is_some(), Field::Enabled),
        (request.title.is_some(), Field::Title),
        (request.url.is_some(), Field::Url),
        (request.token.is_some(), Field::Token),
        (request.activities.is_some(), Field::Activities),
    ] {
        if present {
            fields.push(field);
        }
    }
    let mut redactor = Redactor::with_secret(context.record().token());
    if let Some(token) = &request.token {
        redactor = redactor.and_secret(token);
    }
    let response = context
        .client()
        .update_integration(
            &args.target.board_id,
            &args.target.integration_id,
            &request,
            context.record().token(),
        )
        .await
        .map_err(|e| {
            map_client_error_with_not_found(
                e,
                &redactor,
                "integration update",
                "the Wekan board was not found",
            )
        })?;
    Ok(CommandSuccess::IntegrationUpdated(
        IntegrationUpdateSuccess {
            board_id: args.target.board_id,
            integration_id: response.integration_id,
            submitted_fields: fields,
        },
    ))
}
