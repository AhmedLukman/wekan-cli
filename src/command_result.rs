use serde::Serialize;

#[derive(Debug, Eq, PartialEq)]
pub enum CommandSuccess {
    Cancelled(CancellationSuccess),
    Registration(AuthSuccess),
    Login(AuthSuccess),
    Logout(LogoutSuccess),
    AuthStatus(AuthStatusSuccess),
    UserCurrent(UserDetail),
    UserList(UserListSuccess),
    UserShown(UserDetail),
    UserCards(UserCardsSuccess),
    UserCreated(UserCreateSuccess),
    UserBoards(UserBoardsSuccess),
    UserOwnershipTaken(UserOwnershipSuccess),
    UserLoginChanged(UserLoginChangeSuccess),
    UserDeleted(UserDeleteSuccess),
    ProfileAdded(ProfileItem),
    ProfileList(ProfileListSuccess),
    ProfileShown(ProfileItem),
    ProfileUsed(ProfileItem),
    ProfileUpdated(ProfileItem),
    ProfileRemoved(ProfileRemoveSuccess),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DestructiveOperation {
    AuthLogout,
    ProfileRemove,
    UserTakeOwnership,
    UserDisableLogin,
    UserDelete,
}

impl DestructiveOperation {
    pub const fn as_command(self) -> &'static str {
        match self {
            Self::AuthLogout => "auth logout",
            Self::ProfileRemove => "profile remove",
            Self::UserTakeOwnership => "user take-ownership",
            Self::UserDisableLogin => "user disable-login",
            Self::UserDelete => "user delete",
        }
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CancellationSuccess {
    pub cancelled: bool,
    pub operation: DestructiveOperation,
}

impl CancellationSuccess {
    pub const fn new(operation: DestructiveOperation) -> Self {
        Self {
            cancelled: true,
            operation,
        }
    }
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserBoardSummary {
    pub board_id: String,
    pub title: String,
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
