pub mod auth;
pub mod boards;
pub mod cards;
pub mod comments;
pub mod error;
pub mod lists;
mod raw;
pub mod swimlanes;
mod transport;
pub mod users;

use std::{net::IpAddr, time::Duration};

use reqwest::{Client, redirect::Policy};
use thiserror::Error;
use url::{Host, Url};

pub use auth::{AuthSession, LoginRequest, LogoutRequest, RegisterRequest};
pub use boards::{
    BoardColor, BoardCounts, BoardDocument, BoardPermission, BoardSummary, BoardType,
    BoardWatchLevel, BoardWatcher, CreateBoardRequest, CreateBoardResult, DeleteBoardResult,
    PresentParentTask, RenameBoardResult,
};
pub use cards::{
    CardCustomField, CardCustomFieldValue, CardDependency, CardDocument, CardLocation, CardPoker,
    CardSticker, CardStickerHighlight, CardSummary, CardVote, CreateCardRequest, CreateCardResult,
    DeleteCardResult, UpdateCardRequest, UpdateCardResult,
};
pub use comments::{
    CommentDocument, CommentSummary, CreateCommentRequest, CreateCommentResult, DeleteCommentResult,
};
pub use error::ClientError;
pub use lists::{
    CreateListRequest, CreateListResult, DeleteListResult, ListDocument, ListSummary, ListWipLimit,
    UpdateListRequest, UpdateListResult,
};
pub use raw::RawApiResponse;
pub(crate) use raw::{RawApiAuthentication, RawApiRequest, RawApiRequestBody, RawApiRequestError};
pub use swimlanes::{
    CreateSwimlaneRequest, CreateSwimlaneResult, DeleteSwimlaneResult, SwimlaneDocument,
    SwimlaneSummary, UpdateSwimlaneRequest, UpdateSwimlaneResult,
};
pub use users::{
    CreateUserRequest, CreateUserResult, CurrentUser, CurrentUserBoard, CurrentUserEmail,
    CurrentUserProfile, UserAction, UserActionResult, UserCard, UserCardQuery, UserEmail,
    UserOrganization, UserProfile, UserRecord, UserSummary, UserTeam,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerUrl(Url);

impl ServerUrl {
    pub fn parse(raw: &str) -> Result<Self, ServerUrlError> {
        let mut url = Url::parse(raw).map_err(ServerUrlError::Invalid)?;

        if !matches!(url.scheme(), "http" | "https") {
            return Err(ServerUrlError::UnsupportedScheme(url.scheme().to_owned()));
        }
        if url.host().is_none() || url.cannot_be_a_base() {
            return Err(ServerUrlError::MissingHost);
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(ServerUrlError::EmbeddedCredentials);
        }
        if url.query().is_some() {
            return Err(ServerUrlError::Query);
        }
        if url.fragment().is_some() {
            return Err(ServerUrlError::Fragment);
        }
        if !url.path().ends_with('/') {
            url.path_segments_mut()
                .map_err(|_| ServerUrlError::MissingHost)?
                .push("");
        }

        Ok(Self(url))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn join(&self, path: &str) -> Result<Url, url::ParseError> {
        self.0.join(path)
    }

    fn is_http_loopback(&self) -> bool {
        self.0.scheme() == "http" && is_loopback(&self.0)
    }

    fn enforce_network_policy(&self, allow_insecure_http: bool) -> Result<(), ServerUrlError> {
        if self.0.scheme() == "http" && !allow_insecure_http && !is_loopback(&self.0) {
            return Err(ServerUrlError::InsecureHttp);
        }
        Ok(())
    }
}

fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(address)) => IpAddr::V4(address).is_loopback(),
        Some(Host::Ipv6(address)) => IpAddr::V6(address).is_loopback(),
        None => false,
    }
}

#[derive(Debug, Error)]
pub enum ServerUrlError {
    #[error("the server URL is invalid: {0}")]
    Invalid(#[source] url::ParseError),
    #[error("unsupported server URL scheme `{0}`; use http or https")]
    UnsupportedScheme(String),
    #[error("the server URL must include a host")]
    MissingHost,
    #[error("the server URL must not contain a username or password")]
    EmbeddedCredentials,
    #[error("the server URL must not contain a query string")]
    Query,
    #[error("the server URL must not contain a fragment")]
    Fragment,
    #[error(
        "plaintext HTTP is allowed only for localhost or loopback addresses; use HTTPS or pass --allow-insecure-http"
    )]
    InsecureHttp,
}

pub struct WekanClientFactory {
    server: ServerUrl,
    profile: Option<String>,
    profile_store_namespace: Option<String>,
    allow_insecure_http: bool,
}

impl WekanClientFactory {
    pub fn for_server(server: ServerUrl, allow_insecure_http: bool) -> Self {
        Self {
            server,
            profile: None,
            profile_store_namespace: None,
            allow_insecure_http,
        }
    }

    pub fn for_resolved_profile(
        server: ServerUrl,
        profile: String,
        profile_store_namespace: String,
        allow_insecure_http: bool,
    ) -> Self {
        Self {
            server,
            profile: Some(profile),
            profile_store_namespace: Some(profile_store_namespace),
            allow_insecure_http,
        }
    }

    #[cfg(test)]
    pub(crate) fn new(server: Option<String>, allow_insecure_http: bool) -> Self {
        let server = ServerUrl::parse(
            server
                .as_deref()
                .expect("test client factories require a server identity"),
        )
        .expect("test client factories require a valid server identity");
        Self::for_resolved_profile(
            server,
            "default".to_owned(),
            "test-store".to_owned(),
            allow_insecure_http,
        )
    }

    #[cfg(test)]
    pub(crate) fn for_profile(server: String, profile: String, allow_insecure_http: bool) -> Self {
        let server = ServerUrl::parse(&server)
            .expect("test client factories require a valid server identity");
        Self::for_resolved_profile(
            server,
            profile,
            "test-store".to_owned(),
            allow_insecure_http,
        )
    }

    pub fn profile(&self) -> Option<&str> {
        self.profile.as_deref()
    }

    pub(crate) fn profile_store_namespace(&self) -> Option<&str> {
        self.profile_store_namespace.as_deref()
    }

    pub fn create(&self) -> Result<WekanClient, WekanClientFactoryError> {
        self.server
            .enforce_network_policy(self.allow_insecure_http)?;
        WekanClient::new(self.server.clone()).map_err(WekanClientFactoryError::Build)
    }

    pub(crate) const fn server_identity(&self) -> &ServerUrl {
        &self.server
    }
}

#[derive(Debug, Error)]
pub enum WekanClientFactoryError {
    #[error(transparent)]
    ServerUrl(#[from] ServerUrlError),
    #[error("the HTTP client could not be initialized")]
    Build(#[source] ClientError),
}

#[derive(Clone, Debug)]
pub struct WekanClient {
    server: ServerUrl,
    http: Client,
}

impl WekanClient {
    fn new(server: ServerUrl) -> Result<Self, ClientError> {
        Self::from_builder(server, Client::builder())
    }

    fn from_builder(
        server: ServerUrl,
        builder: reqwest::ClientBuilder,
    ) -> Result<Self, ClientError> {
        let mut builder = builder
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(Policy::none())
            .retry(reqwest::retry::never())
            .user_agent(concat!("wekan-cli/", env!("CARGO_PKG_VERSION")));
        if server.is_http_loopback() {
            builder = builder.no_proxy();
        }
        let http = builder.build().map_err(ClientError::Build)?;
        Ok(Self { server, http })
    }

    pub fn server(&self) -> &ServerUrl {
        &self.server
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Client;
    use secrecy::SecretString;
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::{
        RegisterRequest, ServerUrl, ServerUrlError, WekanClient, WekanClientFactory,
        WekanClientFactoryError,
    };

    #[test]
    fn client_factory_defers_network_permission_until_creation() {
        let server = ServerUrl::parse("http://wekan.example").unwrap();
        let factory = WekanClientFactory::for_server(server.clone(), false);

        assert_eq!(factory.server_identity(), &server);
        assert!(matches!(
            factory.create(),
            Err(WekanClientFactoryError::ServerUrl(
                ServerUrlError::InsecureHttp
            ))
        ));

        assert!(
            WekanClientFactory::for_server(server, true)
                .create()
                .is_ok()
        );
    }

    #[test]
    fn normalizes_server_paths_without_losing_subpaths() {
        let server = ServerUrl::parse("https://example.com/wekan").unwrap();
        assert_eq!(server.as_str(), "https://example.com/wekan/");
        assert_eq!(
            server.join("users/register").unwrap().as_str(),
            "https://example.com/wekan/users/register"
        );
        assert_eq!(
            server.join("users/login").unwrap().as_str(),
            "https://example.com/wekan/users/login"
        );
        assert_eq!(
            server.join("api/user").unwrap().as_str(),
            "https://example.com/wekan/api/user"
        );
    }

    #[test]
    fn normalization_preserves_percent_encoded_path_segments() {
        let server = ServerUrl::parse("https://example.com/tenant%20one/wekan").unwrap();
        assert_eq!(server.as_str(), "https://example.com/tenant%20one/wekan/");
    }

    #[test]
    fn client_factory_permits_http_for_loopback_hosts() {
        for raw in [
            "http://localhost:3000",
            "http://127.0.0.1:3000",
            "http://[::1]:3000",
        ] {
            let server = ServerUrl::parse(raw).unwrap();
            assert!(
                WekanClientFactory::for_server(server, false)
                    .create()
                    .is_ok()
            );
        }
    }

    #[test]
    fn rejects_url_components_that_make_credentials_or_keys_ambiguous() {
        assert!(matches!(
            ServerUrl::parse("https://alice:secret@example.com"),
            Err(ServerUrlError::EmbeddedCredentials)
        ));
        assert!(matches!(
            ServerUrl::parse("https://example.com?tenant=one"),
            Err(ServerUrlError::Query)
        ));
        assert!(matches!(
            ServerUrl::parse("https://example.com#fragment"),
            Err(ServerUrlError::Fragment)
        ));
    }

    #[tokio::test]
    async fn loopback_http_bypasses_a_configured_proxy() {
        let destination = MockServer::start().await;
        let proxy = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/users/register"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "user-1",
                "token": "server-token",
                "tokenExpires": "2030-01-02T03:04:05Z"
            })))
            .expect(1)
            .mount(&destination)
            .await;
        let server = ServerUrl::parse(&destination.uri()).unwrap();
        let builder = Client::builder().proxy(reqwest::Proxy::all(proxy.uri()).unwrap());
        let client = WekanClient::from_builder(server, builder).unwrap();

        client
            .register(&RegisterRequest {
                username: Some("alice".to_owned()),
                email: None,
                password: SecretString::from("test-password".to_owned()),
            })
            .await
            .expect("the loopback request must go directly to its destination");

        destination.verify().await;
        assert!(proxy.received_requests().await.unwrap().is_empty());
    }
}
