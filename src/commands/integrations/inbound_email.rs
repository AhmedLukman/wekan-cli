use crate::{
    client::WekanClientFactory,
    command_result::CommandSuccess,
    commands::client_error::map_client_error,
    credentials::{SecretInputProvider, SecretSource},
    error::AppError,
    redaction::Redactor,
};
use clap::{Args, Subcommand};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Args)]
pub struct InboundEmailArgs {
    #[command(subcommand)]
    pub command: InboundEmailCommand,
}
#[derive(Debug, Subcommand)]
pub enum InboundEmailCommand {
    /// Submit a native provider payload or RFC822 message.
    Submit(PayloadArgs),
    /// Submit a provider's original bounce payload and signature headers.
    Bounce(BounceArgs),
}
#[derive(Debug, Args)]
pub struct InputArgs {
    /// Original request body file, or - for stdin. Bytes are preserved for signatures.
    #[arg(long)]
    file: PathBuf,
    /// JSON file of provider headers, including webhook secrets or signatures.
    #[arg(long)]
    headers_file: Option<PathBuf>,
}
#[derive(Debug, Args)]
pub struct PayloadArgs {
    #[command(flatten)]
    input: InputArgs,
    /// Native request media type, including any multipart boundary.
    #[arg(long, default_value = "application/json")]
    content_type: String,
}
#[derive(Debug, Args)]
pub struct BounceArgs {
    #[arg(value_parser = ["mailgun","sendgrid","postmark","brevo","mailjet","sparkpost","mandrill","resend"])]
    provider: String,
    #[command(flatten)]
    payload: PayloadArgs,
}
pub(crate) async fn dispatch(
    command: InboundEmailCommand,
    factory: &WekanClientFactory,
    secrets: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    let (input, content_type) = match &command {
        InboundEmailCommand::Submit(a) => (&a.input, a.content_type.as_str()),
        InboundEmailCommand::Bounce(a) => (&a.payload.input, a.payload.content_type.as_str()),
    };
    HeaderValue::from_str(content_type)
        .map_err(|_| AppError::invalid_input("invalid --content-type; no request was sent"))?;
    if input.headers_file.as_deref() == Some(std::path::Path::new("-")) {
        return Err(AppError::invalid_input(
            "--headers-file requires a regular file; stdin is reserved for the payload",
        ));
    }
    let values: BTreeMap<String, String> = if let Some(path) = &input.headers_file {
        let input = secrets
            .read_storage_secret(
                SecretSource::File(path.clone()),
                crate::commands::imports::MAX_JSON_BYTES,
            )
            .await?;
        serde_json::from_str(input.expose_secret()).map_err(|_| {
            AppError::invalid_input(
                "header file must be a JSON object of strings; no request was sent",
            )
        })?
    } else {
        BTreeMap::new()
    };
    let protected: Vec<_> = values
        .into_iter()
        .map(|(name, value)| (name, SecretString::from(value)))
        .collect();
    let mut redactor = Redactor::default();
    let mut headers = HeaderMap::new();
    for (name, value) in &protected {
        redactor = redactor.and_secret(value);
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
            AppError::invalid_input("invalid provider header name; no request was sent")
        })?;
        let value = HeaderValue::from_str(value.expose_secret()).map_err(|_| {
            AppError::invalid_input("invalid provider header value; no request was sent")
        })?;
        headers.insert(name, value);
    }
    let bytes = crate::commands::imports::acquire(&input.file).await?;
    let client = factory.create()?;
    let result = match &command {
        InboundEmailCommand::Submit(_) => client
            .submit_inbound_email(bytes, content_type, headers)
            .await
            .map(|response| CommandSuccess::InboundEmailSubmitted(response.into())),
        InboundEmailCommand::Bounce(a) => client
            .submit_email_bounce(&a.provider, bytes, content_type, headers)
            .await
            .map(|response| CommandSuccess::EmailBounceSubmitted(response.into())),
    };
    result.map_err(|e| map_client_error(e, &redactor, "inbound email hook"))
}
