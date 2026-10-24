use super::{
    AttachmentUploadResponse, BackgroundUploadResponse, POST_TIMEOUT, UploadAttachmentRequest,
    UploadBackgroundRequest, decode_attachment, require_success,
};
use crate::client::{ClientError, WekanClient};
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;

impl WekanClient {
    async fn raw_attachment_upload<T: DeserializeOwned>(
        &self,
        operation: &str,
        query: &[(&str, &str)],
        bytes: Vec<u8>,
        token: &SecretString,
    ) -> Result<T, ClientError> {
        let mut endpoint = self.attachment_endpoint(&[operation])?;
        endpoint
            .query_pairs_mut()
            .extend_pairs(query.iter().copied());
        // The raw route selects its transport by Content-Type. File MIME metadata
        // belongs in fileType so JSON files also use the raw byte transport.
        let body = self
            .execute_success_request(
                self.http
                    .post(endpoint)
                    .timeout(POST_TIMEOUT)
                    .header(ACCEPT, "application/json")
                    .header(CONTENT_TYPE, "application/octet-stream")
                    .bearer_auth(token.expose_secret())
                    .body(bytes),
            )
            .await?;
        decode_attachment(&body)
    }
    pub async fn upload_attachment_raw(
        &self,
        request: UploadAttachmentRequest,
        token: &SecretString,
    ) -> Result<AttachmentUploadResponse, ClientError> {
        let mut query = vec![
            ("boardId", request.board_id.as_str()),
            ("swimlaneId", request.swimlane_id.as_str()),
            ("listId", request.list_id.as_str()),
            ("cardId", request.card_id.as_str()),
            ("fileName", request.file_name.as_str()),
        ];
        if let Some(file_type) = &request.file_type {
            query.push(("fileType", file_type));
        }
        let response: AttachmentUploadResponse = self
            .raw_attachment_upload("upload", &query, request.file_data, token)
            .await?;
        require_success(response.success)?;
        Ok(response)
    }
    pub async fn upload_background_raw(
        &self,
        request: UploadBackgroundRequest,
        token: &SecretString,
    ) -> Result<BackgroundUploadResponse, ClientError> {
        let query = vec![
            ("boardId", request.board_id.as_str()),
            ("fileName", request.file_name.as_str()),
            (
                "fileType",
                request.file_type.as_deref().unwrap_or("image/png"),
            ),
        ];
        let response: BackgroundUploadResponse = self
            .raw_attachment_upload("upload-background", &query, request.file_data, token)
            .await?;
        require_success(response.success)?;
        Ok(response)
    }
}
