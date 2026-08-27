use reqwest::{StatusCode, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::{ClientError, WekanClient, transport::embedded_error};

#[derive(Debug)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: SecretString,
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
pub struct UserSummary {
    #[serde(rename = "_id")]
    pub user_id: String,
    pub username: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BoardSummary {
    #[serde(rename = "_id")]
    pub board_id: String,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
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
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub fullname: Option<String>,
}

pub type CurrentUserProfile = UserProfile;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserOrganization {
    pub org_id: Option<String>,
    pub org_display_name: Option<String>,
    pub is_admin: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserTeam {
    pub team_id: Option<String>,
    pub team_display_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
    pub last_connection_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_default_on_null")]
    pub orgs: Vec<UserOrganization>,
    #[serde(default, deserialize_with = "deserialize_default_on_null")]
    pub teams: Vec<UserTeam>,
    #[serde(default, deserialize_with = "deserialize_default_on_null")]
    pub boards: Vec<CurrentUserBoard>,
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
#[serde(rename_all = "camelCase")]
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
    password: &'a str,
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
struct IdResponse {
    #[serde(rename = "_id")]
    id: String,
}

impl WekanClient {
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
            password: request.password.expose_secret(),
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
            message: format!("the {operation} response contained an empty user id"),
            success_status_received: true,
        })
    } else {
        Ok(user)
    }
}

fn decode_success<T: DeserializeOwned>(
    body: &[u8],
    operation: &str,
    require_body: bool,
) -> Result<T, ClientError> {
    if require_body && body.is_empty() {
        return Err(ClientError::Protocol {
            message: format!("the {operation} response body was empty"),
            success_status_received: true,
        });
    }
    let value: Value = serde_json::from_slice(body).map_err(|_| ClientError::Protocol {
        message: format!("the {operation} response was not valid JSON"),
        success_status_received: true,
    })?;
    if let Some(error) = embedded_error(&value, StatusCode::OK)? {
        return Err(error);
    }
    serde_json::from_value(value).map_err(|_| ClientError::Protocol {
        message: format!("the {operation} response had an invalid shape"),
        success_status_received: true,
    })
}
