use secrecy::SecretString;
use wekan_cli::client::{
    LoginRequest, LogoutRequest, RegisterRequest, ServerUrl, WekanClient, WekanClientFactory,
};
use wiremock::MockServer;

#[path = "contract/requests.rs"]
mod requests;
#[path = "contract/responses.rs"]
mod responses;

fn register_request() -> RegisterRequest {
    RegisterRequest {
        username: Some("alice".to_owned()),
        email: None,
        password: SecretString::from("correct horse battery staple".to_owned()),
    }
}

fn login_request() -> LoginRequest {
    LoginRequest {
        username: Some("alice".to_owned()),
        email: None,
        password: SecretString::from("correct horse battery staple".to_owned()),
        code: None,
    }
}

fn status_token() -> SecretString {
    SecretString::from("status-token".to_owned())
}

fn logout_request(all: bool) -> LogoutRequest {
    LogoutRequest { all }
}

fn logout_token() -> SecretString {
    SecretString::from("logout-token".to_owned())
}

fn client(server: &MockServer) -> WekanClient {
    client_from_url(&server.uri())
}

fn client_from_url(raw: &str) -> WekanClient {
    let server = ServerUrl::parse(raw).unwrap();
    WekanClientFactory::for_server(server, false)
        .create()
        .unwrap()
}
