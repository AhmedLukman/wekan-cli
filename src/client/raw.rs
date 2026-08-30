use std::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use reqwest::{
    Body, Method, StatusCode, Url,
    header::{CONTENT_LENGTH, HeaderMap, HeaderValue},
};
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;
use tokio::io::AsyncRead;
use tokio_util::io::ReaderStream;

use super::{ClientError, ServerUrl, WekanClient, WekanClientFactory};

#[derive(Debug)]
pub(crate) enum RawApiAuthentication<'token> {
    Bearer(&'token SecretString),
    QueryToken(&'token SecretString),
    None,
}

pub(crate) enum RawApiRequestBody {
    Text(String),
    Reader(Box<dyn AsyncRead + Send + Unpin>),
}

impl RawApiRequestBody {
    fn into_reqwest_body(self) -> Body {
        match self {
            Self::Text(body) => Body::from(body),
            Self::Reader(reader) => Body::wrap_stream(ReaderStream::new(reader)),
        }
    }
}

pub(crate) struct RawApiRequest<'request> {
    pub(crate) method: Method,
    pub(crate) path: &'request str,
    pub(crate) query: &'request [(String, String)],
    pub(crate) headers: HeaderMap,
    pub(crate) body: Option<RawApiRequestBody>,
    pub(crate) content_length: Option<u64>,
    pub(crate) authentication: RawApiAuthentication<'request>,
    pub(crate) timeout: Option<Duration>,
}

#[derive(Debug, Error)]
pub(crate) enum RawApiRequestError {
    #[error("{0}")]
    InvalidInput(String),
    #[error(transparent)]
    Client(#[from] ClientError),
}

pub struct RawApiResponse {
    identity: u64,
    response: reqwest::Response,
}

impl fmt::Debug for RawApiResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RawApiResponse")
            .field("identity", &self.identity)
            .field("status", &self.status())
            .finish_non_exhaustive()
    }
}

impl PartialEq for RawApiResponse {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
}

impl Eq for RawApiResponse {}

impl RawApiResponse {
    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn headers(&self) -> &HeaderMap {
        self.response.headers()
    }

    pub(crate) async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, ClientError> {
        let status = self.response.status();
        let retry_after_seconds = self
            .response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        self.response
            .chunk()
            .await
            .map(|chunk| chunk.map(|bytes| bytes.to_vec()))
            .map_err(|source| ClientError::ResponseBody {
                status,
                retry_after_seconds,
                source: source.without_url(),
            })
    }
}

impl WekanClientFactory {
    pub(crate) fn validate_raw_api_request(
        &self,
        method: &Method,
        path: &str,
        query: &[(String, String)],
        query_token: bool,
    ) -> Result<(), RawApiRequestError> {
        raw_api_url(&self.server, method, path, query, query_token, None).map(|_| ())
    }
}

impl WekanClient {
    pub(crate) async fn execute_raw_api_request(
        &self,
        request: RawApiRequest<'_>,
    ) -> Result<RawApiResponse, RawApiRequestError> {
        let RawApiRequest {
            method,
            path,
            query,
            headers,
            body,
            content_length,
            authentication,
            timeout,
        } = request;
        let (query_token, bearer_token, query_token_value) = match authentication {
            RawApiAuthentication::Bearer(token) => (false, Some(token), None),
            RawApiAuthentication::QueryToken(token) => (true, None, Some(token.expose_secret())),
            RawApiAuthentication::None => (false, None, None),
        };
        let url = raw_api_url(
            &self.server,
            &method,
            path,
            query,
            query_token,
            query_token_value,
        )?;
        let mut headers = headers;
        if let Some(length) = content_length {
            headers.insert(CONTENT_LENGTH, HeaderValue::from(length));
        }
        let mut request = self.http.request(method, url).headers(headers);
        if let Some(token) = bearer_token {
            request = request.bearer_auth(token.expose_secret());
        }
        if let Some(body) = body {
            request = request.body(body.into_reqwest_body());
        }
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }

        request
            .send()
            .await
            .map(|response| RawApiResponse {
                identity: next_response_identity(),
                response,
            })
            .map_err(|error| {
                RawApiRequestError::Client(ClientError::Transport(error.without_url()))
            })
    }
}

fn raw_api_url(
    server: &ServerUrl,
    method: &Method,
    path: &str,
    query: &[(String, String)],
    query_token: bool,
    query_token_value: Option<&str>,
) -> Result<Url, RawApiRequestError> {
    if !path.starts_with('/') || path.starts_with("//") {
        return Err(RawApiRequestError::InvalidInput(
            "the raw API path must begin with exactly one slash".to_owned(),
        ));
    }
    if path.contains('#') {
        return Err(RawApiRequestError::InvalidInput(
            "the raw API path must not contain a URL fragment".to_owned(),
        ));
    }

    let path_only = path.split_once('?').map_or(path, |(path, _)| path);
    if path_only.contains('\\') {
        return Err(RawApiRequestError::InvalidInput(
            "the raw API path must use forward slashes and must not contain backslashes".to_owned(),
        ));
    }
    if path_only[1..].split('/').any(is_dot_segment) {
        return Err(RawApiRequestError::InvalidInput(
            "the raw API path must not contain dot-segment traversal".to_owned(),
        ));
    }
    if query_token && *method != Method::GET {
        return Err(RawApiRequestError::InvalidInput(
            "--auth-token-query is accepted only for GET requests".to_owned(),
        ));
    }

    let mut url = server.join(&path[1..]).map_err(|error| {
        RawApiRequestError::InvalidInput(format!("the raw API path is invalid: {error}"))
    })?;
    if url.origin() != server.0.origin() || !url.path().starts_with(server.0.path()) {
        return Err(RawApiRequestError::InvalidInput(
            "the raw API path must stay beneath the configured Wekan server base".to_owned(),
        ));
    }

    if url.query_pairs().any(|(name, _)| name == "authToken")
        || query.iter().any(|(name, _)| name == "authToken")
    {
        return Err(RawApiRequestError::InvalidInput(
            "authToken is managed by --auth-token-query and must not be supplied in the path or --query"
                .to_owned(),
        ));
    }
    if !query.is_empty() || query_token_value.is_some() {
        let mut pairs = url.query_pairs_mut();
        for (name, value) in query {
            pairs.append_pair(name, value);
        }
        if let Some(token) = query_token_value {
            pairs.append_pair("authToken", token);
        }
    }
    Ok(url)
}

fn is_dot_segment(segment: &str) -> bool {
    let mut decoded = segment.to_ascii_lowercase();
    while decoded.contains("%2e") {
        decoded = decoded.replace("%2e", ".");
    }
    matches!(decoded.as_str(), "." | "..")
}

fn next_response_identity() -> u64 {
    static NEXT_IDENTITY: AtomicU64 = AtomicU64::new(1);
    NEXT_IDENTITY.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, time::Duration};

    use reqwest::{Method, header::HeaderMap};
    use secrecy::SecretString;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::path};

    use super::{RawApiAuthentication, RawApiRequest, RawApiRequestError};
    use crate::client::{ClientError, WekanClientFactory};

    #[tokio::test]
    async fn token_bearing_request_urls_are_absent_from_errors_and_response_debug() {
        let token = "managed+/token";
        let closed_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let closed_address = closed_listener.local_addr().unwrap();
        drop(closed_listener);
        let factory = WekanClientFactory::for_profile(
            format!("http://{closed_address}/"),
            "default".to_owned(),
            false,
        );
        let client = factory.create().unwrap();
        let query_token = SecretString::from(token.to_owned());
        let error = client
            .execute_raw_api_request(RawApiRequest {
                method: Method::GET,
                path: "/api/query-auth-on-an-unmodeled-route",
                query: &[],
                headers: HeaderMap::new(),
                body: None,
                content_length: None,
                authentication: RawApiAuthentication::QueryToken(&query_token),
                timeout: Some(Duration::from_secs(1)),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            RawApiRequestError::Client(ClientError::Transport(_))
        ));
        let rendered = format!("{error:?}");
        assert!(!rendered.contains(token));
        assert!(!rendered.contains("managed%2B%2Ftoken"));
        assert!(!rendered.contains("authToken"));

        let server = MockServer::start().await;
        Mock::given(path("/api/query-auth-on-an-unmodeled-route"))
            .respond_with(ResponseTemplate::new(200).insert_header("x-secret", token))
            .mount(&server)
            .await;
        let factory = WekanClientFactory::for_profile(
            format!("{}/", server.uri()),
            "default".to_owned(),
            false,
        );
        let client = factory.create().unwrap();
        let query_token = SecretString::from(token.to_owned());
        let response = client
            .execute_raw_api_request(RawApiRequest {
                method: Method::GET,
                path: "/api/query-auth-on-an-unmodeled-route",
                query: &[],
                headers: HeaderMap::new(),
                body: None,
                content_length: None,
                authentication: RawApiAuthentication::QueryToken(&query_token),
                timeout: None,
            })
            .await
            .unwrap();
        let rendered = format!("{response:?}");
        assert!(!rendered.contains(token));
        assert!(!rendered.contains("authToken"));
        assert!(!rendered.contains("x-secret"));
    }
}
