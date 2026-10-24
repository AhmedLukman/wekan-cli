use crate::client::{
    ClientError, WekanClient, datetime::validate_response_date, transport::decode_success,
};
use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct DeletedAttachment {
    pub attachment_id: String,
    pub attachment_name: String,
    pub attachment_type: String,
    pub board_id: String,
    pub swimlane_id: Option<String>,
    pub list_id: Option<String>,
    pub card_id: Option<String>,
    pub deleted_at: String,
    pub deleted_by: Option<String>,
    pub delete_batch_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct SoftDeleteAttachmentResponse {
    pub deleted: bool,
    pub batch_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreAttachmentResponse {
    pub restored: bool,
}

impl WekanClient {
    pub async fn deleted_board_attachments(
        &self,
        board: &str,
        token: &SecretString,
    ) -> Result<Vec<DeletedAttachment>, ClientError> {
        let mut endpoint = self.board_endpoint(board, Some("attachments"))?;
        endpoint
            .path_segments_mut()
            .expect("HTTP URL supports path segments")
            .push("deleted");
        let body = self
            .execute_success_request(
                self.http
                    .get(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let attachments: Vec<DeletedAttachment> =
            decode_success(&body, "deleted board attachments", true)?;
        for attachment in &attachments {
            if attachment.board_id != board || attachment.attachment_id.trim().is_empty() {
                return Err(super::protocol(
                    "the deleted attachment did not match the requested board",
                ));
            }
            validate_response_date(&attachment.deleted_at)
                .map_err(|()| super::protocol("the deleted attachment timestamp is invalid"))?;
        }
        Ok(attachments)
    }
    pub async fn soft_delete_board_attachment(
        &self,
        board: &str,
        attachment: &str,
        token: &SecretString,
    ) -> Result<SoftDeleteAttachmentResponse, ClientError> {
        let mut endpoint = self.board_endpoint(board, Some("attachments"))?;
        endpoint
            .path_segments_mut()
            .expect("HTTP URL supports path segments")
            .push(attachment);
        let body = self
            .execute_success_request(
                self.http
                    .delete(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "board attachment deletion", true)
    }
    pub async fn restore_board_attachment(
        &self,
        board: &str,
        attachment: &str,
        token: &SecretString,
    ) -> Result<RestoreAttachmentResponse, ClientError> {
        let mut endpoint = self.board_endpoint(board, Some("attachments"))?;
        endpoint
            .path_segments_mut()
            .expect("HTTP URL supports path segments")
            .extend([attachment, "restore"]);
        let body = self
            .execute_success_request(
                self.http
                    .post(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "board attachment restore", true)
    }
}
