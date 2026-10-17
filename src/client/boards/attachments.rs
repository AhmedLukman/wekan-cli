use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::client::{ClientError, WekanClient, transport::decode_success};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardAttachment {
    pub attachment_id: String,
    pub attachment_name: String,
    pub attachment_type: String,
    #[serde(default, deserialize_with = "present")]
    pub url: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub url_download: Option<String>,
    pub board_id: String,
    #[serde(default, deserialize_with = "present")]
    pub swimlane_id: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub list_id: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub card_id: Option<String>,
}

fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}

impl WekanClient {
    pub async fn board_attachments(
        &self,
        board: &str,
        token: &SecretString,
    ) -> Result<Vec<BoardAttachment>, ClientError> {
        super::members::validate_id(board)?;
        let body = self
            .execute_success_request(
                self.http
                    .get(self.board_endpoint(board, Some("attachments"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "board attachment list", true)
    }
}
