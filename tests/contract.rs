use secrecy::SecretString;
use wekan_cli::client::{LoginRequest, RegisterRequest, ServerUrl, WekanClient};
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

fn client(server: &MockServer) -> WekanClient {
    let server = ServerUrl::parse(&server.uri(), false).unwrap();
    WekanClient::new(server).unwrap()
}
