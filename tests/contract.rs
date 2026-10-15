#![recursion_limit = "512"]

use secrecy::SecretString;
use wekan_cli::client::{
    BoardColor, BoardPermission, CreateBoardRequest, CreateUserRequest, LoginRequest,
    LogoutRequest, RegisterRequest, ServerUrl, UserCardQuery, WekanClient, WekanClientFactory,
};
use wiremock::MockServer;

#[path = "contract/bulk_cards.rs"]
mod bulk_cards;
#[path = "contract/checklists.rs"]
mod checklists;
#[path = "contract/requests.rs"]
mod requests;
#[path = "contract/responses.rs"]
mod responses;
#[path = "contract/transfers.rs"]
mod transfers;

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

fn user_token() -> SecretString {
    SecretString::from("user-token".to_owned())
}

fn create_user_request() -> CreateUserRequest {
    CreateUserRequest {
        username: "alice".to_owned(),
        email: "alice@example.com".to_owned(),
        password: SecretString::from("new-user-password".to_owned()).into(),
    }
}

fn create_board_request() -> CreateBoardRequest {
    CreateBoardRequest {
        is_admin: true,
        is_active: true,
        title: "Delivery".to_owned(),
        owner: Some("owner/1".to_owned()),
        permission: BoardPermission::Public,
        color: BoardColor::Cleanlight,
        is_no_comments: true,
        is_comment_only: true,
        is_worker: true,
    }
}

fn user_card_query() -> UserCardQuery {
    UserCardQuery {
        due: true,
        from: Some("2026-08-01T00:00:00Z".to_owned()),
        to: Some("2026-08-31T23:59:59Z".to_owned()),
    }
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

#[path = "contract/attachments.rs"]
mod attachments;
#[path = "contract/custom_fields.rs"]
mod custom_fields;
#[path = "contract/dependencies.rs"]
mod dependencies;
#[path = "contract/integrations.rs"]
mod integrations;
#[path = "contract/memberships.rs"]
mod memberships;
