use super::{ClientError, IdResponse, WekanClient, require_non_empty};
use crate::client::transport::decode_success;
use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct TransferSwimlaneRequest {
    #[serde(rename = "toBoardId", skip_serializing_if = "Option::is_none")]
    pub target_board_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Value>,
}
impl WekanClient {
    pub async fn copy_swimlane(
        &self,
        board_id: &str,
        swimlane_id: &str,
        request: &TransferSwimlaneRequest,
        token: &SecretString,
    ) -> Result<String, ClientError> {
        self.transfer_swimlane(board_id, swimlane_id, request, token, "copy")
            .await
    }
    pub async fn move_swimlane(
        &self,
        board_id: &str,
        swimlane_id: &str,
        request: &TransferSwimlaneRequest,
        token: &SecretString,
    ) -> Result<String, ClientError> {
        self.transfer_swimlane(board_id, swimlane_id, request, token, "move")
            .await
    }
    async fn transfer_swimlane(
        &self,
        board_id: &str,
        swimlane_id: &str,
        request: &TransferSwimlaneRequest,
        token: &SecretString,
        action: &str,
    ) -> Result<String, ClientError> {
        let mut endpoint = self.swimlane_endpoint(board_id, Some(swimlane_id))?;
        endpoint
            .path_segments_mut()
            .expect("HTTP endpoint")
            .push(action);
        let body = self
            .execute_success_request(
                self.http
                    .post(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "swimlane transfer", true)?;
        require_non_empty(&response.id, "swimlane transfer", "swimlane id")?;
        Ok(response.id)
    }
}
