use super::{ClientError, WekanClient, require_non_empty};
use crate::client::transport::decode_success;
use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct CopyBoardRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}
impl WekanClient {
    pub async fn copy_board(
        &self,
        board_id: &str,
        request: &CopyBoardRequest,
        token: &SecretString,
    ) -> Result<String, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.board_endpoint(board_id, Some("copy"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let id: String = decode_success(&body, "board copy", true)?;
        require_non_empty(&id, "board copy", "board id")?;
        Ok(id)
    }
}
