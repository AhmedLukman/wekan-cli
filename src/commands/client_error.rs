use reqwest::StatusCode;

use crate::{
    client::ClientError,
    error::{AppError, ErrorCode, ErrorDetails},
    exit_code::StableExitCode,
    redaction::Redactor,
};

pub(crate) fn map_client_error(
    error: ClientError,
    redactor: &Redactor<'_>,
    operation: &str,
    mutation: bool,
) -> AppError {
    map_client_error_with_optional_not_found(error, redactor, operation, mutation, None)
}

pub(crate) fn map_client_error_with_not_found(
    error: ClientError,
    redactor: &Redactor<'_>,
    operation: &str,
    not_found_message: &str,
) -> AppError {
    map_client_error_with_optional_not_found(
        error,
        redactor,
        operation,
        false,
        Some(not_found_message),
    )
}

fn map_client_error_with_optional_not_found(
    error: ClientError,
    redactor: &Redactor<'_>,
    operation: &str,
    mutation: bool,
    not_found_message: Option<&str>,
) -> AppError {
    let (code, message, exit_code, mut details) = match error {
        ClientError::Build(_) => (
            ErrorCode::InternalError,
            format!("the {operation} HTTP client could not be initialized"),
            StableExitCode::Internal,
            ErrorDetails::default(),
        ),
        ClientError::Transport(error) => (
            ErrorCode::TransportError,
            format!("the {operation} request could not be completed: {error}"),
            StableExitCode::Transport,
            ErrorDetails::default(),
        ),
        ClientError::UnexpectedRedirect { status } => (
            ErrorCode::UnexpectedRedirect,
            format!("the {operation} request received an unexpected redirect"),
            StableExitCode::Transport,
            response_error_details(status, None),
        ),
        ClientError::ResponseTooLarge {
            limit_bytes,
            status,
            retry_after_seconds,
        } => response_body_error(
            format!("the {operation} response exceeded the {limit_bytes}-byte limit"),
            status,
            retry_after_seconds,
            operation,
            not_found_message,
        ),
        ClientError::ResponseBody {
            status,
            retry_after_seconds,
            ..
        } => response_body_error(
            format!("the {operation} response body could not be read"),
            status,
            retry_after_seconds,
            operation,
            not_found_message,
        ),
        ClientError::Protocol {
            message,
            success_status_received,
        } => (
            ErrorCode::ProtocolError,
            message,
            StableExitCode::Transport,
            protocol_error_details(success_status_received),
        ),
        ClientError::EmbeddedProtocol {
            http_status,
            server_error,
            server_reason,
            server_message,
            server_error_type,
            server_is_client_safe,
        } => (
            ErrorCode::ProtocolError,
            format!("the {operation} response contained a Wekan error without statusCode"),
            StableExitCode::Server,
            embedded_protocol_error_details(
                http_status,
                server_error,
                server_reason,
                server_message,
                server_error_type,
                server_is_client_safe,
                redactor,
            ),
        ),
        ClientError::Server {
            status,
            server_error,
            server_reason,
            retry_after_seconds,
        } => {
            let code = status_error_code(status.as_u16(), not_found_message.is_some());
            let details = server_error_details(
                status,
                server_error,
                server_reason,
                retry_after_seconds,
                redactor,
            );
            (
                code,
                status_error_message(code, operation, not_found_message),
                StableExitCode::Server,
                details,
            )
        }
        ClientError::EmbeddedServer {
            http_status,
            wekan_status_code,
            server_error,
            server_reason,
        } => {
            let code = status_error_code(wekan_status_code, not_found_message.is_some());
            let details = embedded_server_error_details(
                http_status,
                wekan_status_code,
                server_error,
                server_reason,
                redactor,
            );
            (
                code,
                status_error_message(code, operation, not_found_message),
                StableExitCode::Server,
                details,
            )
        }
    };

    if mutation && mutation_outcome_is_unknown(code, &details) {
        details.outcome_unknown = Some(true);
    }
    AppError::new(code, redactor.redact(&message), exit_code).with_details(details)
}

fn response_body_error(
    message: String,
    status: StatusCode,
    retry_after_seconds: Option<u64>,
    operation: &str,
    not_found_message: Option<&str>,
) -> (ErrorCode, String, StableExitCode, ErrorDetails) {
    if status == StatusCode::OK {
        return (
            ErrorCode::ProtocolError,
            message,
            StableExitCode::Transport,
            response_error_details(status, retry_after_seconds),
        );
    }

    let code = status_error_code(status.as_u16(), not_found_message.is_some());
    (
        code,
        status_error_message(code, operation, not_found_message),
        StableExitCode::Server,
        response_error_details(status, retry_after_seconds),
    )
}

const fn status_error_code(status: u16, map_not_found: bool) -> ErrorCode {
    match status {
        401 => ErrorCode::AuthenticationRejected,
        403 => ErrorCode::PermissionDenied,
        404 if map_not_found => ErrorCode::NotFound,
        _ => ErrorCode::ServerError,
    }
}

fn status_error_message(
    code: ErrorCode,
    operation: &str,
    not_found_message: Option<&str>,
) -> String {
    match code {
        ErrorCode::AuthenticationRejected => {
            "Wekan rejected the stored credential; remove it with `wekan auth logout --local-only --yes`, then log in again"
                .to_owned()
        }
        ErrorCode::PermissionDenied => format!("Wekan denied permission for {operation}"),
        ErrorCode::NotFound => not_found_message
            .expect("not-found mapping requires a dedicated message")
            .to_owned(),
        _ => format!("Wekan rejected the {operation} request"),
    }
}

fn mutation_outcome_is_unknown(code: ErrorCode, details: &ErrorDetails) -> bool {
    match code {
        ErrorCode::AuthenticationRejected | ErrorCode::PermissionDenied | ErrorCode::NotFound => {
            false
        }
        ErrorCode::ServerError => details
            .wekan_status_code
            .or(details.http_status)
            .is_none_or(|status| status >= StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
        ErrorCode::TransportError | ErrorCode::UnexpectedRedirect | ErrorCode::ProtocolError => {
            true
        }
        _ => false,
    }
}

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
