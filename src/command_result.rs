use serde::Serialize;

#[derive(Debug, Eq, PartialEq)]
pub enum CommandSuccess {
    Registration(AuthSuccess),
    Login(AuthSuccess),
    Logout(LogoutSuccess),
    AuthStatus(AuthStatusSuccess),
    ProfileAdded(ProfileItem),
    ProfileList(ProfileListSuccess),
    ProfileShown(ProfileItem),
    ProfileUsed(ProfileItem),
    ProfileUpdated(ProfileItem),
    ProfileRemoved(ProfileRemoveSuccess),
}

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
    pub profile: Option<String>,
    pub logout_scope: LogoutScope,
    pub remote_logout_completed: bool,
    pub credential_stored: bool,
    pub local_credential_removed: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AuthSuccess {
    pub server: String,
    pub profile: Option<String>,
    pub user_id: String,
    pub token_expires: String,
    pub credential_stored: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AuthStatusSuccess {
    pub server: String,
    pub profile: Option<String>,
    pub authenticated: bool,
    pub token_expires: String,
    pub credential_stored: bool,
    pub user: AuthStatusUser,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProfileItem {
    pub name: String,
    pub server: String,
    pub active: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ProfileListSuccess {
    pub active_profile: Option<String>,
    pub profiles: Vec<ProfileItem>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ProfileRemoveSuccess {
    pub name: String,
    pub server: String,
    pub removed: bool,
    pub active_profile: Option<String>,
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
