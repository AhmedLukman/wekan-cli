use super::{ClientError, IdResponse, WekanClient, require_non_empty};
use crate::client::transport::decode_success;
use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Number;

/// Exact card-copy position; the request Option distinguishes omission from null.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CardCopyPosition {
    Number(Number),
    Text(String),
    Null,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CopyCardRequest {
    #[serde(rename = "toBoardId", skip_serializing_if = "Option::is_none")]
    pub target_board_id: Option<String>,
    #[serde(rename = "toSwimlaneId")]
    pub target_swimlane_id: String,
    #[serde(rename = "toListId", skip_serializing_if = "Option::is_none")]
    pub target_list_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<CardCopyPosition>,
}
impl WekanClient {
    pub async fn copy_card(
        &self,
        board_id: &str,
        list_id: &str,
        card_id: &str,
        request: &CopyCardRequest,
        token: &SecretString,
    ) -> Result<String, ClientError> {
        let mut endpoint = self.card_endpoint(board_id, list_id, Some(card_id))?;
        endpoint
            .path_segments_mut()
            .expect("HTTP endpoint")
            .push("copy");
        let body = self
            .execute_success_request(
                self.http
                    .post(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "card copy", true)?;
        require_non_empty(&response.id, "card copy", "card id")?;
        Ok(response.id)
    }
}
