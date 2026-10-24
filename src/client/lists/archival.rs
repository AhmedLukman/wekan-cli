use crate::client::{ClientError, WekanClient, transport::decode_success};
use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ListArchiveResponse {
    #[serde(rename(deserialize = "_id", serialize = "list_id"))]
    pub list_id: String,
    pub archived: bool,
}

impl WekanClient {
    pub async fn set_list_archived(
        &self,
        board_id: &str,
        list_id: &str,
        archived: bool,
        token: &SecretString,
    ) -> Result<ListArchiveResponse, ClientError> {
        let mut endpoint = self.list_endpoint(board_id, Some(list_id))?;
        endpoint
            .path_segments_mut()
            .expect("HTTP URL supports path segments")
            .push(if archived { "archive" } else { "unarchive" });
        let body = self
            .execute_success_request(
                self.http
                    .post(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: ListArchiveResponse = decode_success(&body, "list archive/restore", true)?;
        if response.list_id != list_id || response.archived != archived {
            return Err(super::protocol_error(
                "the list archive response did not match the requested list and state".to_owned(),
            ));
        }
        Ok(response)
    }
}
