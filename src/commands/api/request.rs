use std::{path::PathBuf, str::FromStr, time::Duration as StdDuration};

use clap::{ArgGroup, Args};
use reqwest::{
    Method,
    header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use time::OffsetDateTime;

use crate::{
    client::{
        RawApiAuthentication, RawApiRequest, RawApiRequestBody, RawApiRequestError, WekanClient,
        WekanClientFactory,
    },
    command_result::{
        ApiResponseSuccess, CancellationSuccess, CommandSuccess, DestructiveOperation,
    },
    commands::{
        authenticated::AuthenticatedContext,
        client_error::map_client_error,
        credential_ops::{lock_credential_mutation, map_credential_load_error},
    },
    credentials::CredentialStore,
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    input::{
        ConfirmationArgs, ConfirmationDecision, ConfirmationProvider, ConfirmationRequest,
        PayloadBody, PayloadSource, confirm_or_skip, escape_terminal_text,
    },
    redaction::Redactor,
};

#[derive(Debug, Args)]
#[command(
    group(ArgGroup::new("body_source").args(["body", "body_file", "body_stdin", "json"]).multiple(false)),
    group(ArgGroup::new("authentication_mode").args(["no_auth", "auth_token_query"]).multiple(false))
)]
pub struct RequestArgs {
    /// HTTP method, including extension methods supported by the server.
    #[arg(value_parser = parse_method)]
    method: Method,

    /// Wekan route beginning with one slash, optionally including a query string.
    path: String,

    /// Append an encoded query pair. May be repeated and must contain `=`.
    #[arg(long = "query", value_name = "NAME=VALUE", value_parser = parse_query)]
    query: Vec<QueryPair>,

    /// Add a request header. May be repeated.
    #[arg(short = 'H', long = "header", value_name = "NAME:VALUE", value_parser = parse_header)]
    headers: Vec<RequestHeader>,

    /// Send this UTF-8 text as the request body.
    #[arg(long)]
    body: Option<String>,

    /// Stream the request body from a file.
    #[arg(long, value_name = "PATH")]
    body_file: Option<PathBuf>,

    /// Stream the request body from standard input.
    #[arg(long)]
    body_stdin: bool,

    /// Send a validated JSON value and default Content-Type to application/json.
    #[arg(long, value_name = "JSON")]
    json: Option<String>,

    /// Do not load or inject the selected profile's stored credential.
    #[arg(long)]
    no_auth: bool,

    /// Put the stored token in authToken for a GET request.
    #[arg(long)]
    auth_token_query: bool,

    /// Override the 30-second total request timeout.
    #[arg(long, value_name = "SECONDS", value_parser = parse_timeout)]
    timeout: Option<StdDuration>,

    #[command(flatten)]
    confirmation: ConfirmationArgs,
}

#[derive(Clone, Debug)]
struct QueryPair {
    name: String,
    value: String,
}

#[derive(Clone, Debug)]
struct RequestHeader {
    name: HeaderName,
    value: HeaderValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuthenticationMode {
    Bearer,
    Query,
    None,
}

impl RequestArgs {
    fn authentication_mode(&self) -> AuthenticationMode {
        if self.no_auth {
            AuthenticationMode::None
        } else if self.auth_token_query {
            AuthenticationMode::Query
        } else {
            AuthenticationMode::Bearer
        }
    }

    fn unsafe_method(&self) -> bool {
        !matches!(self.method, Method::GET | Method::HEAD | Method::OPTIONS)
    }

    fn validate(&self) -> Result<(), AppError> {
        if !self.unsafe_method() && self.confirmation.yes() {
            return Err(AppError::invalid_input(
                "--yes is accepted only for raw API methods that may mutate state",
            ));
        }
        if let Some(json) = &self.json {
            serde_json::from_str::<serde_json::Value>(json)
                .map_err(|error| AppError::invalid_input(format!("--json is invalid: {error}")))?;
        }
        Ok(())
    }

    fn auth_token_query(&self) -> bool {
        self.authentication_mode() == AuthenticationMode::Query
    }

    fn query_pairs(&self) -> Vec<(String, String)> {
        self.query
            .iter()
            .map(|pair| (pair.name.clone(), pair.value.clone()))
            .collect()
    }

    fn request_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for header in &self.headers {
            headers.append(header.name.clone(), header.value.clone());
        }
        if self.json.is_some() && !headers.contains_key(CONTENT_TYPE) {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }
        headers
    }

    fn request_body(&self) -> Option<PayloadSource> {
        if let Some(body) = &self.body {
            return Some(PayloadSource::Text(body.clone()));
        }
        if let Some(json) = &self.json {
            return Some(PayloadSource::Text(json.clone()));
        }
        if let Some(path) = &self.body_file {
            return Some(PayloadSource::File(path.clone()));
        }
        self.body_stdin.then_some(PayloadSource::Stdin)
    }
}

pub(super) async fn execute(
    args: RequestArgs,
    client_factory: &WekanClientFactory,
    credential_store: &dyn CredentialStore,
    confirmation: &dyn ConfirmationProvider,
) -> Result<CommandSuccess, AppError> {
    args.validate()?;
    let query = args.query_pairs();
    client_factory
        .validate_raw_api_request(&args.method, &args.path, &query, args.auth_token_query())
        .map_err(|error| map_raw_request_error(error, &Redactor::default(), false))?;
    let unsafe_method = args.unsafe_method();

    if args.authentication_mode() == AuthenticationMode::None {
        let client = client_factory.create()?;
        if confirm_request(&args, confirmation)? == ConfirmationDecision::Cancelled {
            return Ok(cancelled());
        }
        return send_request(
            &args,
            &client,
            client_factory
                .profile()
                .expect("raw API targets are profiles"),
            RawApiAuthentication::None,
            unsafe_method,
        )
        .await;
    }

    let context = AuthenticatedContext::load(client_factory, credential_store)?;
    if unsafe_method {
        let credential_mutation =
            lock_credential_mutation(credential_store, context.credential_target())?;
        if confirm_request(&args, confirmation)? == ConfirmationDecision::Cancelled {
            return Ok(cancelled());
        }
        let guarded_record = credential_mutation
            .load()
            .map_err(map_credential_load_error)?
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::CredentialNotFound,
                    "the credential used to prepare this raw API request is no longer stored",
                    StableExitCode::Credential,
                )
            })?;
        recheck_managed_credential(&context, &guarded_record)?;
        let result = send_managed_request(&args, &context, unsafe_method).await;
        drop(credential_mutation);
        result
    } else {
        send_managed_request(&args, &context, unsafe_method).await
    }
}

fn recheck_managed_credential(
    context: &AuthenticatedContext,
    guarded_record: &crate::credentials::CredentialRecord,
) -> Result<(), AppError> {
    if !guarded_record.matches(context.record()) {
        return Err(AppError::new(
            ErrorCode::CredentialStoreFailed,
            "the stored credential changed while preparing the raw API request; no HTTP request was sent",
            StableExitCode::Credential,
        )
        .with_credential_stored(true));
    }
    if guarded_record.token_expires() <= OffsetDateTime::now_utc() {
        return Err(AppError::new(
            ErrorCode::CredentialExpired,
            format!(
                "the stored credential expired at {}; no HTTP request was sent",
                context.token_expires()
            ),
            StableExitCode::Server,
        )
        .with_details(ErrorDetails {
            token_expires: Some(context.token_expires().to_owned()),
            ..ErrorDetails::default()
        }));
    }
    Ok(())
}

async fn send_managed_request(
    args: &RequestArgs,
    context: &AuthenticatedContext,
    unsafe_method: bool,
) -> Result<CommandSuccess, AppError> {
    let authentication = match args.authentication_mode() {
        AuthenticationMode::Bearer => RawApiAuthentication::Bearer(context.record().token()),
        AuthenticationMode::Query => RawApiAuthentication::QueryToken(context.record().token()),
        AuthenticationMode::None => RawApiAuthentication::None,
    };
    send_request(
        args,
        context.client(),
        context.profile(),
        authentication,
        unsafe_method,
    )
    .await
}

async fn send_request(
    args: &RequestArgs,
    client: &WekanClient,
    profile: &str,
    authentication: RawApiAuthentication<'_>,
    unsafe_method: bool,
) -> Result<CommandSuccess, AppError> {
    let authenticated = !matches!(&authentication, RawApiAuthentication::None);
    let query = args.query_pairs();
    let headers = args.request_headers();
    let payload = match args.request_body() {
        Some(source) => Some(
            source
                .acquire()
                .await
                .map_err(|error| AppError::invalid_input(error.to_string()))?,
        ),
        None => None,
    };
    let (body, content_length) = payload
        .map(|payload| {
            let body = match payload.body {
                PayloadBody::Text(body) => RawApiRequestBody::Text(body),
                PayloadBody::Reader(reader) => RawApiRequestBody::Reader(reader),
            };
            (Some(body), payload.content_length)
        })
        .unwrap_or((None, None));
    let redactor = match &authentication {
        RawApiAuthentication::Bearer(token) | RawApiAuthentication::QueryToken(token) => {
            Redactor::with_secret(token)
        }
        RawApiAuthentication::None => Redactor::default(),
    };
    let response = client
        .execute_raw_api_request(RawApiRequest {
            method: args.method.clone(),
            path: &args.path,
            query: &query,
            headers,
            body,
            content_length,
            authentication,
            timeout: args.timeout,
        })
        .await;
    let response =
        response.map_err(|error| map_raw_request_error(error, &redactor, unsafe_method))?;
    Ok(CommandSuccess::ApiResponse(ApiResponseSuccess {
        profile: profile.to_owned(),
        unsafe_method,
        authenticated,
        response,
    }))
}

fn map_raw_request_error(
    error: RawApiRequestError,
    redactor: &Redactor<'_>,
    unsafe_method: bool,
) -> AppError {
    match error {
        RawApiRequestError::InvalidInput(message) => AppError::invalid_input(message),
        RawApiRequestError::Client(error) => {
            map_client_error(error, redactor, "raw API", unsafe_method)
        }
    }
}

fn confirm_request(
    args: &RequestArgs,
    confirmation: &dyn ConfirmationProvider,
) -> Result<ConfirmationDecision, AppError> {
    if !args.unsafe_method() {
        return Ok(ConfirmationDecision::Proceed);
    }
    let display_path = args
        .path
        .split_once('?')
        .map_or(args.path.as_str(), |(path, _)| path);
    let request = ConfirmationRequest::new(
        DestructiveOperation::ApiRequest,
        format!(
            "Send potentially mutating {} request to `{}`?",
            args.method,
            escape_terminal_text(display_path)
        ),
    );
    confirm_or_skip(&args.confirmation, confirmation, &request)
}

fn cancelled() -> CommandSuccess {
    CommandSuccess::Cancelled(CancellationSuccess::new(DestructiveOperation::ApiRequest))
}

fn parse_method(value: &str) -> Result<Method, String> {
    let method = Method::from_bytes(value.as_bytes())
        .map_err(|_| "method is not a valid HTTP token".to_owned())?;
    if ["TRACE", "TRACK", "CONNECT"]
        .iter()
        .any(|blocked| method.as_str().eq_ignore_ascii_case(blocked))
    {
        return Err(format!(
            "method `{}` is not permitted by the raw API transport",
            method.as_str()
        ));
    }
    Ok(method)
}

fn parse_timeout(value: &str) -> Result<StdDuration, String> {
    let seconds = value
        .parse::<u64>()
        .map_err(|_| "timeout must be a positive integer number of seconds".to_owned())?;
    if seconds == 0 {
        return Err("timeout must be greater than zero seconds".to_owned());
    }
    Ok(StdDuration::from_secs(seconds))
}

fn parse_query(value: &str) -> Result<QueryPair, String> {
    let Some((name, value)) = value.split_once('=') else {
        return Err("query values must use NAME=VALUE".to_owned());
    };
    if name.is_empty() {
        return Err("query parameter names must not be empty".to_owned());
    }
    Ok(QueryPair {
        name: name.to_owned(),
        value: value.to_owned(),
    })
}

fn parse_header(value: &str) -> Result<RequestHeader, String> {
    let Some((name, value)) = value.split_once(':') else {
        return Err("headers must use NAME:VALUE".to_owned());
    };
    let name = HeaderName::from_str(name.trim()).map_err(|_| "invalid header name".to_owned())?;
    if is_managed_header(&name) {
        return Err(format!(
            "header `{name}` is managed by the raw API transport and cannot be overridden"
        ));
    }
    let value = HeaderValue::from_str(value.trim_matches([' ', '\t']))
        .map_err(|_| "invalid header value".to_owned())?;
    Ok(RequestHeader { name, value })
}

fn is_managed_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "authorization"
            | "host"
            | "content-length"
            | "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}
