use reqwest::{StatusCode, header::RETRY_AFTER};
use serde::Deserialize;
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

        let error = serde_json::from_slice::<WekanErrorResponse>(&self.body).unwrap_or_default();
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
    if !value.is_object() {
        return Ok(None);
    }

    let error = serde_json::from_value::<WekanErrorResponse>(value.clone()).unwrap_or_default();
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
    use serde_json::json;

    use super::WekanErrorResponse;

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
}
