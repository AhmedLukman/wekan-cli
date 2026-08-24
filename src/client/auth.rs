use reqwest::{StatusCode, header::ACCEPT};
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthTokenResponse {
    id: String,
    token: String,
    token_expires: String,
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
        let endpoint =
            self.server()
                .join("users/register")
                .map_err(|error| ClientError::Protocol {
                    message: format!("could not build the registration endpoint: {error}"),
                    account_created: Some(false),
                })?;
        let body = RegisterRequestBody {
            username: request.username.as_deref(),
            email: request.email.as_deref(),
            password: request.password.expose_secret(),
        };

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

        let response_body = read_limited_body(response).await?;

        if status != StatusCode::OK {
            let wekan_error =
                serde_json::from_slice::<WekanErrorResponse>(&response_body).unwrap_or_default();
            return Err(ClientError::Server {
                status,
                server_error: wekan_error.error.map(WekanErrorCode::into_string),
                server_reason: wekan_error.reason,
            });
        }

        let response =
            serde_json::from_slice::<AuthTokenResponse>(&response_body).map_err(|_| {
                ClientError::Protocol {
                    message: "invalid JSON or missing fields".to_owned(),
                    account_created: Some(true),
                }
            })?;

        if response.id.is_empty() {
            return Err(ClientError::Protocol {
                message: "the user id was empty".to_owned(),
                account_created: Some(true),
            });
        }
        if response.token.is_empty() {
            return Err(ClientError::Protocol {
                message: "the login token was empty".to_owned(),
                account_created: Some(true),
            });
        }
        let token_expires =
            OffsetDateTime::parse(&response.token_expires, &Rfc3339).map_err(|error| {
                ClientError::Protocol {
                    message: format!("tokenExpires was not RFC 3339: {error}"),
                    account_created: Some(true),
                }
            })?;

        Ok(AuthSession {
            user_id: response.id,
            token: SecretString::from(response.token),
            token_expires,
        })
    }
}

async fn read_limited_body(mut response: reqwest::Response) -> Result<Vec<u8>, ClientError> {
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(ClientError::ResponseTooLarge {
            limit_bytes: MAX_RESPONSE_BYTES,
            status,
        });
    }

    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|source| ClientError::ResponseBody { status, source })?
    {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(ClientError::ResponseTooLarge {
                limit_bytes: MAX_RESPONSE_BYTES,
                status,
            });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
