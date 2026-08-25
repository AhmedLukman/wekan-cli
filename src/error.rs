use serde::Serialize;

use crate::{
    client::{ServerUrlError, WekanClientFactoryError},
    exit_code::StableExitCode,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidInput,
    ConfigurationError,
    InsecureTransport,
    TransportError,
    UnexpectedRedirect,
    ProtocolError,
    LoginRejected,
    LoginRateLimited,
    RegistrationRejected,
    RegistrationDisabled,
    ServerError,
    CredentialStoreUnavailable,
    CredentialStoreFailed,
    InternalError,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::ConfigurationError => "configuration_error",
            Self::InsecureTransport => "insecure_transport",
            Self::TransportError => "transport_error",
            Self::UnexpectedRedirect => "unexpected_redirect",
            Self::ProtocolError => "protocol_error",
            Self::LoginRejected => "login_rejected",
            Self::LoginRateLimited => "login_rate_limited",
            Self::RegistrationRejected => "registration_rejected",
            Self::RegistrationDisabled => "registration_disabled",
            Self::ServerError => "server_error",
            Self::CredentialStoreUnavailable => "credential_store_unavailable",
            Self::CredentialStoreFailed => "credential_store_failed",
            Self::InternalError => "internal_error",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ErrorDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_created: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_created: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub two_factor_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome_unknown: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppError {
    code: ErrorCode,
    message: String,
    exit_code: StableExitCode,
    details: ErrorDetails,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>, exit_code: StableExitCode) -> Self {
        Self {
            code,
            message: message.into(),
            exit_code,
            details: ErrorDetails::default(),
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::InvalidInput,
            message,
            StableExitCode::InvalidInput,
        )
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::ConfigurationError,
            message,
            StableExitCode::Configuration,
        )
    }

    pub fn with_details(mut self, details: ErrorDetails) -> Self {
        self.details = details;
        self
    }

    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub const fn exit_code(&self) -> StableExitCode {
        self.exit_code
    }

    pub const fn details(&self) -> &ErrorDetails {
        &self.details
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

impl From<WekanClientFactoryError> for AppError {
    fn from(error: WekanClientFactoryError) -> Self {
        match error {
            WekanClientFactoryError::MissingServer => Self::configuration(
                "a Wekan server URL is required; pass --server or set WEKAN_URL",
            ),
            WekanClientFactoryError::ServerUrl(error @ ServerUrlError::InsecureHttp) => Self::new(
                ErrorCode::InsecureTransport,
                error.to_string(),
                StableExitCode::Configuration,
            ),
            WekanClientFactoryError::ServerUrl(error) => Self::configuration(error.to_string()),
            WekanClientFactoryError::Build(_) => Self::new(
                ErrorCode::InternalError,
                "the HTTP client could not be initialized",
                StableExitCode::Internal,
            ),
        }
    }
}
