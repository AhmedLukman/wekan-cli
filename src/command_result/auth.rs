use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogoutScope {
    CurrentToken,
    AllTokens,
    LocalOnly,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct LogoutSuccess {
    pub server: String,
    pub profile: String,
    pub logout_scope: LogoutScope,
    pub remote_logout_completed: bool,
    pub credential_stored: bool,
    pub local_credential_removed: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AuthSuccess {
    pub server: String,
    pub profile: String,
    pub user_id: String,
    pub token_expires: String,
    pub credential_stored: bool,
    pub profile_created: bool,
    pub profile_active: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AuthStatusSuccess {
    pub server: String,
    pub profile: String,
    pub authenticated: bool,
    pub token_expires: String,
    pub credential_stored: bool,
    pub user: AuthStatusUser,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AuthStatusUser {
    pub user_id: String,
    pub username: Option<String>,
    pub full_name: Option<String>,
    pub is_admin: Option<bool>,
    pub emails: Vec<AuthStatusEmail>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AuthStatusEmail {
    pub address: Option<String>,
    pub verified: Option<bool>,
}
