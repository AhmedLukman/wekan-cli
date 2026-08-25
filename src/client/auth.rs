use reqwest::{
    StatusCode,
    header::{ACCEPT, RETRY_AFTER},
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{ClientError, WekanClient};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Debug)]
pub struct RegisterRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: SecretString,
}

#[derive(Debug)]
pub struct LoginRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: SecretString,
    pub code: Option<SecretString>,
}

#[derive(Debug)]
pub struct AuthSession {
    user_id: String,
    token: SecretString,
    token_expires: OffsetDateTime,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CurrentUser {
    user_id: String,
    username: Option<String>,
    full_name: Option<String>,
    is_admin: Option<bool>,
    emails: Vec<CurrentUserEmail>,
}

impl CurrentUser {
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn full_name(&self) -> Option<&str> {
        self.full_name.as_deref()
    }

    pub const fn is_admin(&self) -> Option<bool> {
        self.is_admin
    }

    pub fn emails(&self) -> &[CurrentUserEmail] {
        &self.emails
    }

    pub fn into_parts(
        self,
    ) -> (
        String,
        Option<String>,
        Option<String>,
        Option<bool>,
        Vec<CurrentUserEmail>,
    ) {
        (
            self.user_id,
            self.username,
            self.full_name,
            self.is_admin,
            self.emails,
        )
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct CurrentUserEmail {
    address: Option<String>,
    verified: Option<bool>,
}

impl CurrentUserEmail {
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

impl AuthSession {
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn token(&self) -> &SecretString {
        &self.token
    }

    pub const fn token_expires(&self) -> OffsetDateTime {
        self.token_expires
    }

    pub fn into_parts(self) -> (String, SecretString, OffsetDateTime) {
        (self.user_id, self.token, self.token_expires)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterRequestBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<&'a str>,
    password: &'a str,
}

#[derive(Serialize)]
struct LoginRequestBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<&'a str>,
    password: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'a str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthTokenResponse {
    id: String,
    token: String,
    token_expires: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurrentUserResponse {
    #[serde(rename = "_id")]
    user_id: Option<String>,
    username: Option<String>,
    #[serde(default)]
    emails: Vec<CurrentUserEmail>,
    profile: Option<CurrentUserProfile>,
    is_admin: Option<bool>,
    error: Option<WekanErrorCode>,
    reason: Option<String>,
    status_code: Option<i64>,
}

#[derive(Deserialize)]
struct CurrentUserProfile {
    fullname: Option<String>,
}

#[derive(Default, Deserialize)]
struct WekanErrorResponse {
    error: Option<WekanErrorCode>,
    reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum WekanErrorCode {
    String(String),
    Number(serde_json::Number),
}

impl WekanErrorCode {
    fn into_string(self) -> String {
        match self {
            Self::String(value) => value,
            Self::Number(value) => value.to_string(),
        }
    }
}

impl WekanClient {
    pub async fn register(&self, request: &RegisterRequest) -> Result<AuthSession, ClientError> {
        let body = RegisterRequestBody {
            username: request.username.as_deref(),
            email: request.email.as_deref(),
            password: request.password.expose_secret(),
        };

        self.authenticate("users/register", "registration", &body)
            .await
    }

    pub async fn login(&self, request: &LoginRequest) -> Result<AuthSession, ClientError> {
        let body = LoginRequestBody {
            username: request.username.as_deref(),
            email: request.email.as_deref(),
            password: request.password.expose_secret(),
            code: request.code.as_ref().map(ExposeSecret::expose_secret),
        };

        self.authenticate("users/login", "login", &body).await
    }

    pub async fn current_user(&self, token: &SecretString) -> Result<CurrentUser, ClientError> {
        let endpoint = self
            .server()
            .join("api/user")
            .map_err(|error| ClientError::Protocol {
                message: format!("could not build the current-user endpoint: {error}"),
                success_status_received: false,
            })?;

        let response = self
            .http
            .get(endpoint)
            .header(ACCEPT, "application/json")
            .bearer_auth(token.expose_secret())
            .send()
            .await
            .map_err(ClientError::Transport)?;
        let status = response.status();

        if status.is_redirection() {
            return Err(ClientError::UnexpectedRedirect { status });
        }

        let retry_after_seconds = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let response_body = read_limited_body(response, retry_after_seconds).await?;

        if status != StatusCode::OK {
            let wekan_error =
                serde_json::from_slice::<WekanErrorResponse>(&response_body).unwrap_or_default();
            return Err(ClientError::Server {
                status,
                server_error: wekan_error.error.map(WekanErrorCode::into_string),
                server_reason: wekan_error.reason,
                retry_after_seconds,
            });
        }

        let response =
            serde_json::from_slice::<CurrentUserResponse>(&response_body).map_err(|_| {
                ClientError::Protocol {
                    message: "invalid JSON or invalid current-user fields".to_owned(),
                    success_status_received: true,
                }
            })?;

        if let Some(status_code) = response.status_code {
            let wekan_status_code =
                u16::try_from(status_code).map_err(|_| ClientError::Protocol {
                    message: "the embedded Wekan statusCode was outside the valid range".to_owned(),
                    success_status_received: true,
                })?;
            return Err(ClientError::EmbeddedServer {
                http_status: status,
                wekan_status_code,
                server_error: response.error.map(WekanErrorCode::into_string),
                server_reason: response.reason,
            });
        }

        if response.error.is_some() || response.reason.is_some() {
            return Err(ClientError::Protocol {
                message: "the current-user response contained an error without statusCode"
                    .to_owned(),
                success_status_received: true,
            });
        }

        let user_id = response.user_id.ok_or_else(|| ClientError::Protocol {
            message: "the current-user response did not contain a user id".to_owned(),
            success_status_received: true,
        })?;
        if user_id.is_empty() {
            return Err(ClientError::Protocol {
                message: "the current-user response contained an empty user id".to_owned(),
                success_status_received: true,
            });
        }

        Ok(CurrentUser {
            user_id,
            username: response.username,
            full_name: response.profile.and_then(|profile| profile.fullname),
            is_admin: response.is_admin,
            emails: response.emails,
        })
    }

    async fn authenticate(
        &self,
        path: &str,
        operation: &str,
        body: &impl Serialize,
    ) -> Result<AuthSession, ClientError> {
        let endpoint = self
            .server()
            .join(path)
            .map_err(|error| ClientError::Protocol {
                message: format!("could not build the {operation} endpoint: {error}"),
                success_status_received: false,
            })?;

        let response = self
            .http
            .post(endpoint)
            .header(ACCEPT, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(ClientError::Transport)?;
        let status = response.status();

        if status.is_redirection() {
            return Err(ClientError::UnexpectedRedirect { status });
        }

        let retry_after_seconds = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let response_body = read_limited_body(response, retry_after_seconds).await?;

        if status != StatusCode::OK {
            let wekan_error =
                serde_json::from_slice::<WekanErrorResponse>(&response_body).unwrap_or_default();
            return Err(ClientError::Server {
                status,
                server_error: wekan_error.error.map(WekanErrorCode::into_string),
                server_reason: wekan_error.reason,
                retry_after_seconds,
            });
        }

        let response =
            serde_json::from_slice::<AuthTokenResponse>(&response_body).map_err(|_| {
                ClientError::Protocol {
                    message: "invalid JSON or missing fields".to_owned(),
                    success_status_received: true,
                }
            })?;

        if response.id.is_empty() {
            return Err(ClientError::Protocol {
                message: "the user id was empty".to_owned(),
                success_status_received: true,
            });
        }
        if response.token.is_empty() {
            return Err(ClientError::Protocol {
                message: "the login token was empty".to_owned(),
                success_status_received: true,
            });
        }
        let token_expires =
            OffsetDateTime::parse(&response.token_expires, &Rfc3339).map_err(|error| {
                ClientError::Protocol {
                    message: format!("tokenExpires was not RFC 3339: {error}"),
                    success_status_received: true,
                }
            })?;

        Ok(AuthSession {
            user_id: response.id,
            token: SecretString::from(response.token),
            token_expires,
        })
    }
}

async fn read_limited_body(
    mut response: reqwest::Response,
    retry_after_seconds: Option<u64>,
) -> Result<Vec<u8>, ClientError> {
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(ClientError::ResponseTooLarge {
            limit_bytes: MAX_RESPONSE_BYTES,
            status,
            retry_after_seconds,
        });
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|source| ClientError::ResponseBody {
            status,
            retry_after_seconds,
            source,
        })?
    {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(ClientError::ResponseTooLarge {
                limit_bytes: MAX_RESPONSE_BYTES,
                status,
                retry_after_seconds,
            });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
