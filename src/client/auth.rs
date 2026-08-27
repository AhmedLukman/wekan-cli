use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{ClientError, WekanClient};

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

#[derive(Debug, Serialize)]
pub struct LogoutRequest {
    pub all: bool,
}

#[derive(Debug)]
pub struct AuthSession {
    user_id: String,
    token: SecretString,
    token_expires: OffsetDateTime,
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
struct LogoutResponse {
    message: String,
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

    pub async fn logout(
        &self,
        request: &LogoutRequest,
        token: &SecretString,
    ) -> Result<(), ClientError> {
        let endpoint =
            self.server()
                .join("users/logout")
                .map_err(|error| ClientError::Protocol {
                    message: format!("could not build the logout endpoint: {error}"),
                    success_status_received: false,
                })?;

        let response_body = self
            .execute_success_request(
                self.http
                    .post(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;

        let response = serde_json::from_slice::<LogoutResponse>(&response_body).map_err(|_| {
            ClientError::Protocol {
                message: "invalid JSON or missing logout message".to_owned(),
                success_status_received: true,
            }
        })?;
        let _ = response.message;
        Ok(())
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

        let response_body = self
            .execute_success_request(
                self.http
                    .post(endpoint)
                    .header(ACCEPT, "application/json")
                    .json(&body),
            )
            .await?;

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
