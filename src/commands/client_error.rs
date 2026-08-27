use reqwest::StatusCode;

use crate::{error::ErrorDetails, redaction::Redactor};

pub(crate) fn protocol_error_details(success_status_received: bool) -> ErrorDetails {
    ErrorDetails {
        http_status: success_status_received.then_some(StatusCode::OK.as_u16()),
        ..ErrorDetails::default()
    }
}

pub(crate) fn response_error_details(
    status: StatusCode,
    retry_after_seconds: Option<u64>,
) -> ErrorDetails {
    ErrorDetails {
        http_status: Some(status.as_u16()),
        retry_after_seconds: retry_after_for_status(status, retry_after_seconds),
        ..ErrorDetails::default()
    }
}

fn retry_after_for_status(status: StatusCode, retry_after_seconds: Option<u64>) -> Option<u64> {
    (status == StatusCode::TOO_MANY_REQUESTS)
        .then_some(retry_after_seconds)
        .flatten()
}

pub(crate) fn server_error_details(
    status: StatusCode,
    server_error: Option<String>,
    server_reason: Option<String>,
    retry_after_seconds: Option<u64>,
    redactor: &Redactor<'_>,
) -> ErrorDetails {
    let mut details = response_error_details(status, retry_after_seconds);
    details.server_error = server_error.map(|value| redactor.redact(&value));
    details.server_reason = server_reason.map(|value| redactor.redact(&value));
    details
}

pub(crate) fn embedded_server_error_details(
    http_status: StatusCode,
    wekan_status_code: u16,
    server_error: Option<String>,
    server_reason: Option<String>,
    redactor: &Redactor<'_>,
) -> ErrorDetails {
    ErrorDetails {
        http_status: Some(http_status.as_u16()),
        wekan_status_code: Some(wekan_status_code),
        server_error: server_error.map(|value| redactor.redact(&value)),
        server_reason: server_reason.map(|value| redactor.redact(&value)),
        ..ErrorDetails::default()
    }
}

pub(crate) fn embedded_protocol_error_details(
    http_status: StatusCode,
    server_error: Option<String>,
    server_reason: Option<String>,
    server_message: Option<String>,
    server_error_type: Option<String>,
    server_is_client_safe: Option<bool>,
    redactor: &Redactor<'_>,
) -> ErrorDetails {
    ErrorDetails {
        http_status: Some(http_status.as_u16()),
        server_error: server_error.map(|value| redactor.redact(&value)),
        server_reason: server_reason.map(|value| redactor.redact(&value)),
        server_message: server_message.map(|value| redactor.redact(&value)),
        server_error_type: server_error_type.map(|value| redactor.redact(&value)),
        server_is_client_safe,
        ..ErrorDetails::default()
    }
}
