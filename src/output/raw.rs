use std::{io::Write, process::ExitCode};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use reqwest::{StatusCode, header::HeaderMap};

use crate::{
    client::{ClientError, RawApiResponse},
    command_result::{
        ApiResponseSuccess, RawApiResponseData, RawResponseBody, RawResponseHeader,
        RawValueEncoding,
    },
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
};

const MAX_STRUCTURED_RESPONSE_BYTES: usize = 1024 * 1024;

pub(super) async fn write_response(
    format: super::OutputFormat,
    response: ApiResponseSuccess,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<ExitCode, AppError> {
    if format == super::OutputFormat::Raw {
        return write_raw_response(response, stdout, stderr).await;
    }

    let ApiResponseSuccess {
        profile,
        unsafe_method,
        authenticated,
        mut response,
    } = response;
    let status = response.status();
    let raw_headers = response.headers().clone();
    let mut body = Vec::new();
    loop {
        let chunk = match response.next_chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(error) => {
                let error = api_response_body_error(error, profile, unsafe_method, authenticated);
                writeln!(stderr, "{}", super::render_error(format, &error))
                    .map_err(super::output_write_error)?;
                return Ok(error.exit_code().into());
            }
        };
        if body.len().saturating_add(chunk.len()) > MAX_STRUCTURED_RESPONSE_BYTES {
            let error = api_response_too_large(status, profile, unsafe_method);
            writeln!(stderr, "{}", super::render_error(format, &error))
                .map_err(super::output_write_error)?;
            return Ok(error.exit_code().into());
        }
        body.extend_from_slice(&chunk);
    }

    let data = structured_api_response(status, &raw_headers, body, unsafe_method);
    if status.is_success() {
        let rendered = match format {
            super::OutputFormat::Human => render_human_response(&data),
            super::OutputFormat::Json => super::render_json_success(&data),
            super::OutputFormat::Raw => unreachable!(),
        };
        writeln!(stdout, "{rendered}").map_err(super::output_write_error)?;
        return Ok(ExitCode::SUCCESS);
    }

    let error = api_status_error(
        status,
        &raw_headers,
        profile,
        unsafe_method,
        authenticated,
        data,
    );
    writeln!(stderr, "{}", super::render_error(format, &error))
        .map_err(super::output_write_error)?;
    Ok(error.exit_code().into())
}

async fn write_raw_response(
    response: ApiResponseSuccess,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<ExitCode, AppError> {
    let ApiResponseSuccess { mut response, .. } = response;
    let status = response.status();
    match write_raw_api_body(&mut response, status, stdout, stderr).await {
        Ok(()) => {}
        Err(RawResponseWriteError::Response) => {
            return Ok(StableExitCode::Transport.into());
        }
        Err(RawResponseWriteError::Output) => {
            return Ok(StableExitCode::Transport.into());
        }
    }
    Ok(if status.is_success() {
        ExitCode::SUCCESS
    } else if status.is_redirection() {
        StableExitCode::Transport.into()
    } else {
        StableExitCode::Server.into()
    })
}

#[derive(Debug)]
enum RawResponseWriteError {
    Response,
    Output,
}

async fn write_raw_api_body(
    response: &mut RawApiResponse,
    status: StatusCode,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), RawResponseWriteError> {
    loop {
        let chunk = response
            .next_chunk()
            .await
            .map_err(|_| RawResponseWriteError::Response)?;
        let Some(chunk) = chunk else {
            break;
        };
        if status.is_success() {
            stdout
                .write_all(&chunk)
                .map_err(|_| RawResponseWriteError::Output)?;
        } else {
            stderr
                .write_all(&chunk)
                .map_err(|_| RawResponseWriteError::Output)?;
        }
    }
    if status.is_success() {
        stdout.flush().map_err(|_| RawResponseWriteError::Output)
    } else {
        stderr.flush().map_err(|_| RawResponseWriteError::Output)
    }
}

pub(crate) fn structured_api_response(
    status: StatusCode,
    headers: &HeaderMap,
    body: Vec<u8>,
    unsafe_method: bool,
) -> RawApiResponseData {
    let headers = headers
        .iter()
        .map(|(name, value)| match value.to_str() {
            Ok(value) => RawResponseHeader {
                name: name.as_str().to_owned(),
                encoding: RawValueEncoding::Text,
                value: value.to_owned(),
            },
            Err(_) => RawResponseHeader {
                name: name.as_str().to_owned(),
                encoding: RawValueEncoding::Base64,
                value: BASE64.encode(value.as_bytes()),
            },
        })
        .collect();
    let body = match serde_json::from_slice::<serde_json::Value>(&body) {
        Ok(value) => RawResponseBody::Json { value },
        Err(_) => match String::from_utf8(body) {
            Ok(value) => RawResponseBody::Text { value },
            Err(error) => RawResponseBody::Base64 {
                value: BASE64.encode(error.into_bytes()),
            },
        },
    };
    RawApiResponseData {
        http_status: status.as_u16(),
        mutation_attempted: unsafe_method,
        mutation_confirmed: false,
        headers,
        body,
    }
}

pub(crate) fn render_human_response(data: &RawApiResponseData) -> String {
    let status = StatusCode::from_u16(data.http_status)
        .expect("response status codes are valid HTTP status codes");
    let reason = status.canonical_reason().unwrap_or("");
    let mut lines = vec![if reason.is_empty() {
        format!("HTTP {}", data.http_status)
    } else {
        format!("HTTP {} {reason}", data.http_status)
    }];
    if data.mutation_attempted {
        lines.push("Mutation attempted: yes".to_owned());
        lines.push("Mutation confirmed: no (no authoritative readback)".to_owned());
    }
    lines.extend(data.headers.iter().map(|header| match header.encoding {
        RawValueEncoding::Text => format!(
            "{}: {}",
            header.name,
            super::escape_terminal_controls(&header.value)
        ),
        RawValueEncoding::Base64 => format!("{}: [base64] {}", header.name, header.value),
    }));
    lines.push(String::new());
    lines.push(match &data.body {
        RawResponseBody::Json { value } => {
            serde_json::to_string_pretty(value).expect("JSON response bodies always serialize")
        }
        RawResponseBody::Text { value } => super::escape_terminal_controls(value),
        RawResponseBody::Base64 { value } => format!(
            "<{} binary bytes; use --output raw for exact bytes>",
            BASE64.decode(value).map_or(0, |bytes| bytes.len())
        ),
    });
    lines.join("\n")
}

fn api_response_body_error(
    error: ClientError,
    profile: String,
    unsafe_method: bool,
    authenticated: bool,
) -> AppError {
    let ClientError::ResponseBody {
        status,
        retry_after_seconds,
        ..
    } = error
    else {
        return AppError::new(
            ErrorCode::InternalError,
            "the raw API response failed with an unexpected client error",
            StableExitCode::Internal,
        )
        .with_profile_context(profile);
    };

    api_response_body_error_for_status(
        status,
        retry_after_seconds,
        profile,
        unsafe_method,
        authenticated,
    )
}

fn api_response_body_error_for_status(
    status: StatusCode,
    retry_after_seconds: Option<u64>,
    profile: String,
    unsafe_method: bool,
    authenticated: bool,
) -> AppError {
    let success = status.is_success();
    let retry_warning = if success && unsafe_method {
        " Wekan already returned HTTP success; do not retry this mutation."
    } else {
        ""
    };
    let (code, message, exit_code) = if success {
        (
            ErrorCode::ProtocolError,
            format!("the raw API response body could not be read.{retry_warning}"),
            StableExitCode::Transport,
        )
    } else if status.is_redirection() {
        (
            ErrorCode::UnexpectedRedirect,
            "the raw API request received an unexpected redirect".to_owned(),
            StableExitCode::Transport,
        )
    } else if status == StatusCode::UNAUTHORIZED {
        (
            ErrorCode::AuthenticationRejected,
            if authenticated {
                "Wekan rejected the stored credential; remove it with `wekan auth logout --local-only --yes`, then log in again".to_owned()
            } else {
                "Wekan rejected the unauthenticated raw API request".to_owned()
            },
            StableExitCode::Server,
        )
    } else if status == StatusCode::FORBIDDEN {
        (
            ErrorCode::PermissionDenied,
            "Wekan denied permission for raw API response".to_owned(),
            StableExitCode::Server,
        )
    } else {
        (
            ErrorCode::ServerError,
            "Wekan rejected the raw API response request".to_owned(),
            StableExitCode::Server,
        )
    };
    AppError::new(code, message, exit_code).with_details(ErrorDetails {
        profile: Some(profile),
        http_status: Some(status.as_u16()),
        http_success: Some(success),
        retry_safe: (success && unsafe_method).then_some(false),
        mutation_attempted: unsafe_method.then_some(true),
        mutation_confirmed: unsafe_method.then_some(false),
        retry_after_seconds: (status == StatusCode::TOO_MANY_REQUESTS)
            .then_some(retry_after_seconds)
            .flatten(),
        outcome_unknown: unsafe_method.then_some(true),
        ..ErrorDetails::default()
    })
}

fn api_status_error(
    status: StatusCode,
    headers: &HeaderMap,
    profile: String,
    unsafe_method: bool,
    authenticated: bool,
    response: RawApiResponseData,
) -> AppError {
    let (code, message, exit_code) = if status.is_redirection() {
        (
            ErrorCode::UnexpectedRedirect,
            "the raw API request received an unexpected redirect".to_owned(),
            StableExitCode::Transport,
        )
    } else if status == StatusCode::UNAUTHORIZED {
        (
            ErrorCode::AuthenticationRejected,
            if authenticated {
                "Wekan rejected the stored credential; remove it with `wekan auth logout --local-only --yes`, then log in again".to_owned()
            } else {
                "Wekan rejected the unauthenticated raw API request".to_owned()
            },
            StableExitCode::Server,
        )
    } else if status == StatusCode::FORBIDDEN {
        (
            ErrorCode::PermissionDenied,
            "Wekan denied permission for the raw API request".to_owned(),
            StableExitCode::Server,
        )
    } else {
        (
            ErrorCode::ServerError,
            "Wekan rejected the raw API request".to_owned(),
            StableExitCode::Server,
        )
    };
    let retry_after_seconds = (status == StatusCode::TOO_MANY_REQUESTS)
        .then(|| {
            headers
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
        })
        .flatten();
    AppError::new(code, message, exit_code).with_details(ErrorDetails {
        profile: Some(profile),
        http_status: Some(status.as_u16()),
        http_success: Some(false),
        mutation_attempted: unsafe_method.then_some(true),
        mutation_confirmed: unsafe_method.then_some(false),
        retry_after_seconds,
        outcome_unknown: (unsafe_method && (status.is_redirection() || status.is_server_error()))
            .then_some(true),
        response: Some(response),
        ..ErrorDetails::default()
    })
}

fn api_response_too_large(status: StatusCode, profile: String, unsafe_method: bool) -> AppError {
    let success = status.is_success();
    let retry_warning = if success && unsafe_method {
        " Wekan already returned HTTP success; do not retry this mutation."
    } else {
        ""
    };
    AppError::new(
        ErrorCode::ApiResponseTooLarge,
        format!(
            "the raw API response exceeded the {MAX_STRUCTURED_RESPONSE_BYTES}-byte structured-output limit; rerun with --output raw.{retry_warning}"
        ),
        StableExitCode::Transport,
    )
    .with_details(ErrorDetails {
        profile: Some(profile),
        http_status: Some(status.as_u16()),
        http_success: Some(success),
        response_limit_bytes: Some(MAX_STRUCTURED_RESPONSE_BYTES as u64),
        retry_safe: (success && unsafe_method).then_some(false),
        mutation_attempted: unsafe_method.then_some(true),
        mutation_confirmed: unsafe_method.then_some(false),
        outcome_unknown: unsafe_method.then_some(true),
        ..ErrorDetails::default()
    })
}

#[cfg(test)]
mod tests {
    use super::{api_response_body_error_for_status, api_response_too_large};
    use crate::{error::ErrorCode, exit_code::StableExitCode};
    use reqwest::StatusCode;

    #[test]
    fn response_body_failures_preserve_status_classification_and_retry_safety() {
        let created = api_response_body_error_for_status(
            StatusCode::CREATED,
            None,
            "default".to_owned(),
            true,
            true,
        );
        assert_eq!(created.code(), ErrorCode::ProtocolError);
        assert_eq!(created.exit_code(), StableExitCode::Transport);
        assert_eq!(created.details().http_status, Some(201));
        assert_eq!(created.details().http_success, Some(true));
        assert_eq!(created.details().retry_safe, Some(false));
        assert_eq!(created.details().outcome_unknown, Some(true));
        assert_eq!(created.details().mutation_attempted, Some(true));
        assert_eq!(created.details().mutation_confirmed, Some(false));
        assert!(created.message().contains("do not retry"));

        let redirect = api_response_body_error_for_status(
            StatusCode::FOUND,
            None,
            "default".to_owned(),
            true,
            true,
        );
        assert_eq!(redirect.code(), ErrorCode::UnexpectedRedirect);
        assert_eq!(redirect.exit_code(), StableExitCode::Transport);
        assert_eq!(redirect.details().http_status, Some(302));
        assert_eq!(redirect.details().http_success, Some(false));
        assert_eq!(redirect.details().outcome_unknown, Some(true));
        assert_eq!(redirect.details().mutation_attempted, Some(true));
        assert_eq!(redirect.details().mutation_confirmed, Some(false));

        let bad_request = api_response_body_error_for_status(
            StatusCode::BAD_REQUEST,
            None,
            "default".to_owned(),
            true,
            true,
        );
        assert_eq!(bad_request.details().outcome_unknown, Some(true));

        let rate_limited =
            api_response_too_large(StatusCode::TOO_MANY_REQUESTS, "default".to_owned(), true);
        assert_eq!(rate_limited.details().outcome_unknown, Some(true));
    }

    #[test]
    fn unauthorized_body_failure_respects_authentication_mode() {
        let unauthenticated = api_response_body_error_for_status(
            StatusCode::UNAUTHORIZED,
            None,
            "default".to_owned(),
            false,
            false,
        );
        assert_eq!(unauthenticated.code(), ErrorCode::AuthenticationRejected);
        assert!(unauthenticated.message().contains("unauthenticated"));
        assert!(!unauthenticated.message().contains("stored credential"));

        let authenticated = api_response_body_error_for_status(
            StatusCode::UNAUTHORIZED,
            None,
            "default".to_owned(),
            false,
            true,
        );
        assert_eq!(authenticated.code(), ErrorCode::AuthenticationRejected);
        assert!(authenticated.message().contains("stored credential"));
    }
}
