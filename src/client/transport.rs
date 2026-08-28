use reqwest::{StatusCode, header::RETRY_AFTER};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

use super::{ClientError, WekanClient};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct WekanErrorResponse {
    error: Option<WekanErrorCode>,
    reason: Option<String>,
    message: Option<String>,
    error_type: Option<String>,
    is_client_safe: Option<bool>,
    status_code: Option<i64>,
    #[serde(rename = "success")]
    _success: Option<bool>,
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

struct ResponseBody {
    status: StatusCode,
    retry_after_seconds: Option<u64>,
    body: Vec<u8>,
}

impl ResponseBody {
    fn require_status(self, expected: StatusCode) -> Result<Vec<u8>, ClientError> {
        if self.status == expected {
            return Ok(self.body);
        }

        let error = match serde_json::from_slice::<Value>(&self.body) {
            Ok(value) => decode_error_response(
                value,
                "the Wekan error response had an invalid shape",
                false,
            )?,
            Err(_) => WekanErrorResponse::default(),
        };
        Err(ClientError::Server {
            status: self.status,
            server_error: error.error.map(WekanErrorCode::into_string),
            server_reason: error.reason,
            retry_after_seconds: self.retry_after_seconds,
        })
    }
}

impl WekanClient {
    pub(super) async fn execute_success_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<Vec<u8>, ClientError> {
        self.execute_request(request)
            .await?
            .require_status(StatusCode::OK)
    }

    async fn execute_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<ResponseBody, ClientError> {
        let response = request.send().await.map_err(ClientError::Transport)?;
        let status = response.status();
        if status.is_redirection() {
            return Err(ClientError::UnexpectedRedirect { status });
        }

        let retry_after_seconds = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let body = read_limited_body(response, retry_after_seconds).await?;

        Ok(ResponseBody {
            status,
            retry_after_seconds,
            body,
        })
    }
}

pub(super) fn embedded_error(
    value: &Value,
    http_status: StatusCode,
) -> Result<Option<ClientError>, ClientError> {
    let Some(object) = value.as_object() else {
        return Ok(None);
    };

    if !["statusCode", "error", "reason", "message", "errorType"]
        .iter()
        .any(|field| object.contains_key(*field))
    {
        return Ok(None);
    }

    let error = decode_error_response(
        value.clone(),
        "the embedded Wekan error response had an invalid shape",
        http_status.is_success(),
    )?;
    if let Some(status_code) = error.status_code {
        let wekan_status_code = u16::try_from(status_code).map_err(|_| ClientError::Protocol {
            message: "the embedded Wekan statusCode was outside the valid range".to_owned(),
            success_status_received: true,
        })?;
        return Ok(Some(ClientError::EmbeddedServer {
            http_status,
            wekan_status_code,
            server_error: error.error.map(WekanErrorCode::into_string),
            server_reason: error.reason,
        }));
    }
    if error.error.is_some()
        || error.reason.is_some()
        || error.message.is_some()
        || error.error_type.is_some()
    {
        return Ok(Some(ClientError::EmbeddedProtocol {
            http_status,
            server_error: error.error.map(WekanErrorCode::into_string),
            server_reason: error.reason,
            server_message: error.message,
            server_error_type: error.error_type,
            server_is_client_safe: error.is_client_safe,
        }));
    }
    Ok(None)
}

fn decode_error_response(
    value: Value,
    message: &str,
    success_status_received: bool,
) -> Result<WekanErrorResponse, ClientError> {
    serde_json::from_value(value).map_err(|_| ClientError::Protocol {
        message: message.to_owned(),
        success_status_received,
    })
}

pub(super) fn decode_success<T: DeserializeOwned>(
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

#[cfg(test)]
mod strict_response_tests {
    use reqwest::StatusCode;
    use serde_json::json;

    use super::{ResponseBody, WekanErrorResponse, embedded_error};
    use crate::client::ClientError;

    #[test]
    fn wekan_error_response_rejects_unmapped_fields() {
        assert!(
            serde_json::from_value::<WekanErrorResponse>(json!({
                "isClientSafe": true,
                "error": "forbidden",
                "reason": "denied",
                "message": "denied [forbidden]",
                "errorType": "Meteor.Error",
                "statusCode": 403,
                "success": false
            }))
            .is_ok()
        );
        assert!(
            serde_json::from_value::<WekanErrorResponse>(json!({
                "error": "forbidden",
                "unexpected": true
            }))
            .is_err()
        );
    }

    #[test]
    fn non_success_json_errors_reject_unmapped_and_invalid_fields() {
        for body in [
            json!({"error": "forbidden", "unexpected": true}),
            json!({"error": ["wrong", "type"]}),
        ] {
            let error = ResponseBody {
                status: StatusCode::BAD_REQUEST,
                retry_after_seconds: None,
                body: serde_json::to_vec(&body).unwrap(),
            }
            .require_status(StatusCode::OK)
            .unwrap_err();

            assert!(matches!(
                error,
                ClientError::Protocol {
                    success_status_received: false,
                    ..
                }
            ));
        }
    }

    #[test]
    fn unreadable_non_success_error_bodies_retain_status_classification() {
        let error = ResponseBody {
            status: StatusCode::BAD_GATEWAY,
            retry_after_seconds: None,
            body: b"not json".to_vec(),
        }
        .require_status(StatusCode::OK)
        .unwrap_err();

        assert!(matches!(
            error,
            ClientError::Server {
                status: StatusCode::BAD_GATEWAY,
                server_error: None,
                server_reason: None,
                ..
            }
        ));
    }

    #[test]
    fn embedded_errors_reject_unmapped_and_invalid_fields() {
        for value in [
            json!({
                "statusCode": 404,
                "error": "not-found",
                "unexpected": true
            }),
            json!({"statusCode": "404", "error": "not-found"}),
        ] {
            assert!(matches!(
                embedded_error(&value, StatusCode::OK),
                Err(ClientError::Protocol {
                    success_status_received: true,
                    ..
                })
            ));
        }

        assert!(
            embedded_error(&json!({"_id": "user-1"}), StatusCode::OK)
                .unwrap()
                .is_none()
        );
    }
}
