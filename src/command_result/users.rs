use super::UserBoardSummary;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserDetail {
    pub user_id: String,
    pub username: Option<String>,
    pub full_name: Option<String>,
    pub emails: Vec<UserEmail>,
    pub is_admin: Option<bool>,
    pub login_disabled: Option<bool>,
    pub authentication_method: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub last_connection_date: Option<String>,
    pub organizations: Vec<UserOrganization>,
    pub teams: Vec<UserTeam>,
    pub boards: Vec<UserBoardMembership>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserEmail {
    pub address: Option<String>,
    pub verified: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserOrganization {
    pub organization_id: Option<String>,
    pub display_name: Option<String>,
    pub is_admin: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserTeam {
    pub team_id: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserBoardMembership {
    pub board_id: String,
    pub is_active: Option<bool>,
    pub is_admin: Option<bool>,
    pub is_no_comments: Option<bool>,
    pub is_comment_only: Option<bool>,
    pub is_worker: Option<bool>,
    pub is_normal_assigned_only: Option<bool>,
    pub is_comment_assigned_only: Option<bool>,
    pub is_read_only: Option<bool>,
    pub is_read_assigned_only: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserSummary {
    pub user_id: String,
    pub username: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserListSuccess {
    pub users: Vec<UserSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserCard {
    pub card_id: String,
    pub title: Option<String>,
    pub board_id: Option<String>,
    pub swimlane_id: Option<String>,
    pub list_id: Option<String>,
    pub due_at: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub members: Vec<String>,
    pub assignees: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserCardsSuccess {
    pub cards: Vec<UserCard>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserBoardsSuccess {
    pub user_id: String,
    pub boards: Vec<UserBoardSummary>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserCreateWarning {
    #[serde(rename = "user_id_unavailable_in_wekan_v11_06")]
    UserIdUnavailableInWekanV1106,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserCreateSuccess {
    pub created: bool,
    pub username: String,
    pub email: String,
    pub user_id: Option<String>,
    pub warning: Option<UserCreateWarning>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserOwnershipSuccess {
    pub from_user_id: String,
    pub to_user_id: String,
    pub boards: Vec<UserBoardSummary>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserLoginAction {
    Disabled,
    Enabled,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserLoginChangeSuccess {
    pub action: UserLoginAction,
    pub user: UserDetail,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserDeleteSuccess {
    pub user_id: String,
    pub deleted: bool,
    pub deleted_current_user: bool,
    pub credential_stored: bool,
    pub local_credential_removed: bool,
}
