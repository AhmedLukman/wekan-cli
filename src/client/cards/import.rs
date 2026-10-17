use super::{ClientError, WekanClient};
use crate::client::imports::PreparedImportRequest;
use crate::client::transport::decode_success;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
pub struct IcsImportRequest {
    pub ics: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct IcsImportResponse {
    pub created: u64,
    pub card_ids: Vec<String>,
}

impl WekanClient {
    pub async fn import_ics(
        &self,
        board_id: &str,
        swimlane_id: &str,
        list_id: &str,
        request: PreparedImportRequest<IcsImportRequest>,
        token: &SecretString,
        timeout: Option<Duration>,
    ) -> Result<IcsImportResponse, ClientError> {
        let mut url = self.board_endpoint(board_id, None)?;
        url.path_segments_mut()
            .expect("board endpoint is a base URL")
            .extend(["swimlanes", swimlane_id, "lists", list_id, "ics"]);
        let mut request = self
            .http
            .post(url)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .bearer_auth(token.expose_secret())
            .body(request.into_body());
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        let body = self.execute_success_request(request).await?;
        decode_success(&body, "ICS import", true)
    }
}
