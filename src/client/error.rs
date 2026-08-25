use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("failed to initialize the HTTP client")]
    Build(#[source] reqwest::Error),
    #[error("the request could not be completed")]
    Transport(#[source] reqwest::Error),
    #[error("the server returned an unexpected redirect ({status})")]
    UnexpectedRedirect { status: StatusCode },
    #[error("the server response exceeded the {limit_bytes}-byte limit")]
    ResponseTooLarge {
        limit_bytes: usize,
        status: StatusCode,
        retry_after_seconds: Option<u64>,
    },
    #[error("the server response body could not be read")]
    ResponseBody {
        status: StatusCode,
        retry_after_seconds: Option<u64>,
        #[source]
        source: reqwest::Error,
    },
    #[error("the server returned an invalid success response: {message}")]
    Protocol {
        message: String,
        success_status_received: bool,
    },
    #[error("the Wekan server returned an error ({status})")]
    Server {
        status: StatusCode,
        server_error: Option<String>,
        server_reason: Option<String>,
        retry_after_seconds: Option<u64>,
    },
    #[error(
        "the Wekan server returned an embedded error ({wekan_status_code}) in an HTTP {http_status} response"
    )]
    EmbeddedServer {
        http_status: StatusCode,
        wekan_status_code: u16,
        server_error: Option<String>,
        server_reason: Option<String>,
    },
}
