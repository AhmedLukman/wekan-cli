use std::collections::BTreeMap;

pub mod tokens;

use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Number, Value};

use super::{ClientError, WekanClient, boards::BoardSummary, transport::decode_success};

#[derive(Debug)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: UserPassword,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UserAction {
    TakeOwnership,
    DisableLogin,
    EnableLogin,
}

#[derive(Debug, Eq, PartialEq)]
pub enum UserActionResult {
    OwnershipTransferred(Vec<BoardSummary>),
    LoginChanged(Box<UserRecord>),
}

impl UserAction {
    const fn as_wire_value(self) -> &'static str {
        match self {
            Self::TakeOwnership => "takeOwnership",
            Self::DisableLogin => "disableLogin",
            Self::EnableLogin => "enableLogin",
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct UserCardQuery {
    pub due: bool,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CreateUserResult;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UserSummary {
    #[serde(rename = "_id")]
    pub user_id: String,
    pub username: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UserEmail {
    pub address: Option<String>,
    pub verified: Option<bool>,
}

impl UserEmail {
    pub fn address(&self) -> Option<&str> {
        self.address.as_deref()
    }

    pub const fn verified(&self) -> Option<bool> {
        self.verified
    }

    pub fn into_parts(self) -> (Option<String>, Option<bool>) {
        (self.address, self.verified)
    }
}

pub type CurrentUserEmail = UserEmail;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UserProfile {
    pub fullname: Option<String>,
    pub avatar_url: Option<String>,
    pub email_buffer: Option<Vec<Option<String>>>,
    pub show_desktop_drag_handles: Option<bool>,
    pub left_menu_collapsed: Option<bool>,
    pub collapsed_workspaces: Option<BTreeMap<String, bool>>,
    pub left_menu_width: Option<Number>,
    pub submit_on_enter: Option<bool>,
    pub open_many_cards_at_once: Option<bool>,
    pub all_boards_theme_tiles: Option<bool>,
    pub global_theme_color: Option<String>,
    pub global_theme_custom_colors: Option<Vec<Option<String>>>,
    pub ui_font: Option<String>,
    pub ui_font_size: Option<String>,
    pub ui_text_color: Option<String>,
    pub ui_text_bg_color: Option<String>,
    pub dismissed_announcement_version: Option<String>,
    pub card_maximized: Option<bool>,
    pub card_collapsed: Option<bool>,
    pub show_activities: Option<bool>,
    pub custom_fields_grid: Option<bool>,
    pub trello_api_saved: Option<bool>,
    pub hidden_minicard_label_text: Option<bool>,
    pub initials: Option<String>,
    pub board_workspaces_tree: Option<Vec<UserWorkspaceNode>>,
    pub board_workspace_assignments: Option<BTreeMap<String, String>>,
    pub invited_boards: Option<Vec<Option<String>>>,
    pub language: Option<String>,
    pub map_provider: Option<String>,
    pub move_and_copy_dialog: Option<BTreeMap<String, Value>>,
    pub move_checklist_dialog: Option<BTreeMap<String, Value>>,
    pub copy_checklist_dialog: Option<BTreeMap<String, Value>>,
    pub notifications: Option<Vec<UserProfileNotification>>,
    pub rescue_card_description: Option<bool>,
    pub show_cards_count_at: Option<Number>,
    pub start_day_of_week: Option<Number>,
    pub starred_boards: Option<Vec<Option<String>>>,
    pub starred_pages: Option<Vec<UserProfileStarredPage>>,
    pub default_board_id: Option<String>,
    pub icode: Option<String>,
    pub board_view: Option<String>,
    pub list_sort_by: Option<String>,
    pub all_boards_sort_by: Option<String>,
    pub templates_board_id: Option<String>,
    pub card_templates_swimlane_id: Option<String>,
    pub list_templates_swimlane_id: Option<String>,
    pub board_templates_swimlane_id: Option<String>,
    pub sidebar_width: Option<Number>,
    pub list_widths: Option<BTreeMap<String, BTreeMap<String, Number>>>,
    pub list_constraints: Option<BTreeMap<String, BTreeMap<String, Value>>>,
    pub auto_width_boards: Option<BTreeMap<String, bool>>,
    pub board_sort_index: Option<BTreeMap<String, Number>>,
    pub fixed_list_width_boards: Option<BTreeMap<String, bool>>,
    pub fixed_list_widths: Option<BTreeMap<String, Number>>,
    pub swimlane_heights: Option<BTreeMap<String, BTreeMap<String, Number>>>,
    pub collapsed_lists: Option<BTreeMap<String, BTreeMap<String, bool>>>,
    pub collapsed_swimlanes: Option<BTreeMap<String, BTreeMap<String, bool>>>,
    pub collapsed_card_sections: Option<BTreeMap<String, BTreeMap<String, bool>>>,
    pub keyboard_shortcuts: Option<bool>,
    pub vertical_scrollbars: Option<bool>,
    pub show_week_of_year: Option<bool>,
    pub date_format: Option<String>,
    pub mobile_mode: Option<bool>,
    pub card_zoom: Option<Number>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UserWorkspaceNode {
    pub id: Option<String>,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub children: Option<Vec<UserWorkspaceNode>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UserProfileNotification {
    pub activity: String,
    pub read: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UserProfileStarredPage {
    pub url: String,
    pub title: String,
}

pub type CurrentUserProfile = UserProfile;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UserOrganization {
    pub org_id: Option<String>,
    pub org_display_name: Option<String>,
    pub is_admin: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UserTeam {
    pub team_id: Option<String>,
    pub team_display_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CurrentUserBoard {
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UserRecord {
    #[serde(rename = "_id")]
    pub user_id: String,
    pub username: Option<String>,
    #[serde(default, deserialize_with = "deserialize_default_on_null")]
    pub emails: Vec<UserEmail>,
    #[serde(default, deserialize_with = "deserialize_default_on_null")]
    pub profile: UserProfile,
    pub is_admin: Option<bool>,
    pub login_disabled: Option<bool>,
    pub authentication_method: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub created_through_api: Option<bool>,
    pub heartbeat: Option<String>,
    pub last_connection_date: Option<String>,
    pub session_data: Option<UserSessionData>,
    #[serde(default, deserialize_with = "deserialize_default_on_null")]
    pub import_usernames: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_default_on_null")]
    pub orgs: Vec<UserOrganization>,
    #[serde(default, deserialize_with = "deserialize_default_on_null")]
    pub teams: Vec<UserTeam>,
    #[serde(default, deserialize_with = "deserialize_default_on_null")]
    pub boards: Vec<CurrentUserBoard>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UserSessionData {
    pub total_hits: Option<Number>,
}

impl UserRecord {
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn full_name(&self) -> Option<&str> {
        self.profile.fullname.as_deref()
    }

    pub const fn is_admin(&self) -> Option<bool> {
        self.is_admin
    }

    pub fn emails(&self) -> &[UserEmail] {
        &self.emails
    }

    pub fn into_auth_parts(
        self,
    ) -> (
        String,
        Option<String>,
        Option<String>,
        Option<bool>,
        Vec<UserEmail>,
    ) {
        (
            self.user_id,
            self.username,
            self.profile.fullname,
            self.is_admin,
            self.emails,
        )
    }
}

pub type CurrentUser = UserRecord;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UserCard {
    #[serde(rename = "_id")]
    pub card_id: String,
    pub title: Option<String>,
    pub board_id: Option<String>,
    pub swimlane_id: Option<String>,
    pub list_id: Option<String>,
    pub due_at: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_default_on_null")]
    pub members: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_default_on_null")]
    pub assignees: Vec<String>,
}

fn deserialize_default_on_null<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Serialize)]
struct CreateUserRequestBody<'a> {
    username: &'a str,
    email: &'a str,
    password: &'a UserPassword,
}

#[derive(Serialize)]
struct UserActionRequestBody {
    action: &'static str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CreateUserV1106Response {
    #[serde(rename = "_id")]
    _id: EmptyObject,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyObject {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdResponse {
    #[serde(rename = "_id")]
    id: String,
}

impl WekanClient {
    pub async fn update_user_without_action(
        &self,
        user_id: &str,
        token: &SecretString,
    ) -> Result<UserRecord, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.user_endpoint(user_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(&serde_json::json!({})),
            )
            .await?;
        validate_user_record(decode_success(&body, "user update", true)?, "user update")
    }
    pub async fn current_user(&self, token: &SecretString) -> Result<CurrentUser, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.endpoint("api/user", "current-user")?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let user = decode_success(&body, "current-user", true)?;
        validate_user_record(user, "current-user")
    }

    pub async fn user_cards(
        &self,
        query: &UserCardQuery,
        token: &SecretString,
    ) -> Result<Vec<UserCard>, ClientError> {
        let mut endpoint = self.endpoint("api/user/cards", "current-user cards")?;
        {
            let mut pairs = endpoint.query_pairs_mut();
            if query.due {
                pairs.append_pair("due", "true");
            }
            if let Some(from) = &query.from {
                pairs.append_pair("from", from);
            }
            if let Some(to) = &query.to {
                pairs.append_pair("to", to);
            }
        }
        let body = self
            .execute_success_request(
                self.http
                    .get(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "current-user cards", true)
    }

    pub async fn users(&self, token: &SecretString) -> Result<Vec<UserSummary>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.endpoint("api/users", "users")?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "users", true)
    }

    pub async fn user(
        &self,
        selector: &str,
        token: &SecretString,
    ) -> Result<UserRecord, ClientError> {
        let endpoint = self.user_endpoint(selector, None)?;
        let body = self
            .execute_success_request(
                self.http
                    .get(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let user = decode_success(&body, "user", true)?;
        validate_user_record(user, "user")
    }

    pub async fn create_user(
        &self,
        request: &CreateUserRequest,
        token: &SecretString,
    ) -> Result<CreateUserResult, ClientError> {
        let body = CreateUserRequestBody {
            username: &request.username,
            email: &request.email,
            password: &request.password,
        };
        let response = self
            .execute_success_request(
                self.http
                    .post(self.endpoint("api/users", "create-user")?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(&body),
            )
            .await?;
        let _: CreateUserV1106Response = decode_success(&response, "create-user", true)?;
        Ok(CreateUserResult)
    }

    pub async fn user_boards(
        &self,
        user_id: &str,
        token: &SecretString,
    ) -> Result<Vec<BoardSummary>, ClientError> {
        let endpoint = self.user_endpoint(user_id, Some("boards"))?;
        let body = self
            .execute_success_request(
                self.http
                    .get(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "user boards", true)
    }

    pub async fn user_action(
        &self,
        user_id: &str,
        action: UserAction,
        token: &SecretString,
    ) -> Result<UserActionResult, ClientError> {
        let endpoint = self.user_endpoint(user_id, None)?;
        let body = self
            .execute_success_request(
                self.http
                    .put(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(&UserActionRequestBody {
                        action: action.as_wire_value(),
                    }),
            )
            .await?;
        match action {
            UserAction::TakeOwnership => Ok(UserActionResult::OwnershipTransferred(
                decode_success(&body, "take ownership", true)?,
            )),
            UserAction::DisableLogin | UserAction::EnableLogin => {
                let user = decode_success(&body, "login action", true)?;
                Ok(UserActionResult::LoginChanged(Box::new(
                    validate_user_record(user, "login action")?,
                )))
            }
        }
    }

    pub async fn delete_user(
        &self,
        user_id: &str,
        token: &SecretString,
    ) -> Result<String, ClientError> {
        let endpoint = self.user_endpoint(user_id, None)?;
        let body = self
            .execute_success_request(
                self.http
                    .delete(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "delete-user", true)?;
        if response.id.is_empty() {
            return Err(ClientError::Protocol {
                diagnostic: None,
                message: "the delete-user response contained an empty id".to_owned(),
                success_status_received: true,
            });
        }
        Ok(response.id)
    }

    fn endpoint(&self, path: &str, operation: &str) -> Result<reqwest::Url, ClientError> {
        self.server()
            .join(path)
            .map_err(|error| ClientError::Protocol {
                diagnostic: None,
                message: format!("could not build the {operation} endpoint: {error}"),
                success_status_received: false,
            })
    }

    fn user_endpoint(
        &self,
        selector: &str,
        suffix: Option<&str>,
    ) -> Result<reqwest::Url, ClientError> {
        let mut endpoint = self.endpoint("api/users", "user")?;
        endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                diagnostic: None,
                message: "could not add the user selector to the endpoint".to_owned(),
                success_status_received: false,
            })?
            .push(selector);
        if let Some(suffix) = suffix {
            endpoint
                .path_segments_mut()
                .expect("an HTTP endpoint with a host supports path segments")
                .push(suffix);
        }
        Ok(endpoint)
    }
}

fn validate_user_record(user: UserRecord, operation: &str) -> Result<UserRecord, ClientError> {
    if user.user_id.is_empty() {
        Err(ClientError::Protocol {
            diagnostic: None,
            message: format!("the {operation} response contained an empty user id"),
            success_status_received: true,
        })
    } else {
        Ok(user)
    }
}

#[cfg(test)]
mod strict_response_tests {
    use serde_json::json;

    use super::{IdResponse, UserCard, UserRecord, UserSummary};

    #[test]
    fn user_responses_accept_mapped_dynamic_properties() {
        let user = serde_json::from_value::<UserRecord>(json!({
            "_id": "user-1",
            "profile": {
                "fullname": "Alice",
                "collapsedWorkspaces": {"workspace-1": true},
                "boardWorkspaceAssignments": {"board-1": "workspace-1"},
                "moveAndCopyDialog": {"boardId": "board-1"},
                "listWidths": {"board-1": {"list-1": 320}},
                "listConstraints": {"board-1": {"list-1": "unlimited"}}
            }
        }))
        .unwrap();

        assert_eq!(user.user_id(), "user-1");
        assert_eq!(user.full_name(), Some("Alice"));
    }

    #[test]
    fn user_responses_reject_unmapped_top_level_and_nested_fields() {
        assert!(
            serde_json::from_value::<UserRecord>(json!({
                "_id": "user-1",
                "unexpected": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<UserRecord>(json!({
                "_id": "user-1",
                "profile": {"unexpected": true}
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<UserSummary>(json!({
                "_id": "user-1",
                "unexpected": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<UserCard>(json!({
                "_id": "card-1",
                "unexpected": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<IdResponse>(json!({
                "_id": "user-1",
                "unexpected": true
            }))
            .is_err()
        );
    }
}

#[derive(Clone, Debug)]
pub enum UserPassword {
    Plaintext(SecretString),
    Sha256(SecretString),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PasswordDigestError {
    #[error("invalid password digest object")]
    InvalidObject,
    #[error("password digest must use sha-256 and contain 64 UTF-16 code units")]
    InvalidDigest,
}
impl From<SecretString> for UserPassword {
    fn from(value: SecretString) -> Self {
        Self::Plaintext(value)
    }
}
impl UserPassword {
    pub fn secret(&self) -> &SecretString {
        match self {
            Self::Plaintext(secret) | Self::Sha256(secret) => secret,
        }
    }
    pub fn from_digest_json(input: &SecretString) -> Result<Self, PasswordDigestError> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Digest {
            digest: String,
            algorithm: String,
        }
        let value: Digest = serde_json::from_str(input.expose_secret())
            .map_err(|_| PasswordDigestError::InvalidObject)?;
        if value.algorithm != "sha-256" || value.digest.encode_utf16().count() != 64 {
            return Err(PasswordDigestError::InvalidDigest);
        }
        Ok(Self::Sha256(SecretString::from(value.digest)))
    }
}
impl Serialize for UserPassword {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Plaintext(value) => serializer.serialize_str(value.expose_secret()),
            Self::Sha256(value) => {
                use serde::ser::SerializeStruct;
                let mut object = serializer.serialize_struct("PasswordDigest", 2)?;
                object.serialize_field("digest", value.expose_secret())?;
                object.serialize_field("algorithm", "sha-256")?;
                object.end()
            }
        }
    }
}
