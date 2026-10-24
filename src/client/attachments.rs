use std::time::Duration;

mod raw_upload;
pub mod recovery;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{Method, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::{
    ClientError, WekanClient,
    decoding::{decode_value, parse_json},
    transport::embedded_error,
};

pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
const DOWNLOAD_RESPONSE_BYTES: usize = 96 * 1024 * 1024;
const POST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct AttachmentMetadata {
    pub attachment_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_type: String,
    pub storage_backend: String,
    pub board_id: String,
    #[serde(default, deserialize_with = "optional")]
    pub swimlane_id: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    pub list_id: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    pub card_id: Option<String>,
    #[serde(default, deserialize_with = "optional_date")]
    pub created_at: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    pub is_image: Option<bool>,
}

// Optional contract properties may be absent, but explicit null is not a mapped type.
fn optional<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

fn optional_date<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let value = String::deserialize(d)?;
    super::datetime::validate_response_date(&value)
        .map_err(|()| serde::de::Error::custom("invalid attachment date-time"))?;
    Ok(Some(value))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct AttachmentVersion {
    pub version_name: String,
    #[serde(default, deserialize_with = "optional")]
    pub storage: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    pub size: Option<u64>,
    #[serde(default, deserialize_with = "optional")]
    pub r#type: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct AttachmentInfo {
    #[serde(skip_serializing)]
    pub success: bool,
    pub attachment_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_type: String,
    pub storage_backend: String,
    #[serde(default, deserialize_with = "optional")]
    pub board_id: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    pub swimlane_id: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    pub list_id: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    pub card_id: Option<String>,
    pub versions: Vec<AttachmentVersion>,
    #[serde(default, deserialize_with = "optional_date")]
    pub created_at: Option<String>,
    #[serde(default, deserialize_with = "optional")]
    pub is_image: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AttachmentListResponse {
    pub success: bool,
    pub attachments: Vec<AttachmentMetadata>,
    pub count: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct AttachmentDownloadResponse {
    #[serde(skip_serializing)]
    pub success: bool,
    pub attachment_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_type: String,
    pub storage_backend: String,
    #[serde(skip_serializing)]
    pub base64_data: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BackgroundDownloadResponse {
    #[serde(skip_serializing)]
    pub success: bool,
    pub attachment_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_type: String,
    pub storage_backend: String,
    #[serde(skip_serializing)]
    pub base64_data: String,
    #[serde(rename(deserialize = "backgroundImageURL", serialize = "background_image_url"))]
    pub background_image_url: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadAttachmentRequest {
    pub board_id: String,
    pub swimlane_id: String,
    pub list_id: String,
    pub card_id: String,
    pub file_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<String>,
    /// Raw file bytes; serialized as Wekan's base64 fileData string.
    #[serde(serialize_with = "serialize_file_data")]
    pub file_data: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadBackgroundRequest {
    pub board_id: String,
    pub file_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_type: Option<String>,
    /// Raw file bytes; serialized as Wekan's base64 fileData string.
    #[serde(serialize_with = "serialize_file_data")]
    pub file_data: Vec<u8>,
}

fn serialize_file_data<S: serde::Serializer>(
    bytes: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&STANDARD.encode(bytes))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferAttachmentRequest {
    pub attachment_id: String,
    pub target_board_id: String,
    pub target_swimlane_id: String,
    pub target_list_id: String,
    pub target_card_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct AttachmentUploadResponse {
    #[serde(skip_serializing)]
    pub success: bool,
    pub attachment_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub storage_backend: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BackgroundUploadResponse {
    #[serde(skip_serializing)]
    pub success: bool,
    pub attachment_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub storage_backend: String,
    pub message: String,
    #[serde(rename(deserialize = "backgroundImageURL", serialize = "background_image_url"))]
    pub background_image_url: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct AttachmentCopyResponse {
    #[serde(skip_serializing)]
    pub success: bool,
    pub source_attachment_id: String,
    pub new_attachment_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct AttachmentMoveResponse {
    #[serde(skip_serializing)]
    pub success: bool,
    pub attachment_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub source_board_id: String,
    pub target_board_id: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct AttachmentDeleteResponse {
    #[serde(skip_serializing)]
    pub success: bool,
    pub attachment_id: String,
    pub message: String,
    pub file_name: String,
}

pub(crate) fn protocol(message: &str) -> ClientError {
    ClientError::Protocol {
        message: message.to_owned(),
        diagnostic: None,
        success_status_received: true,
    }
}

pub fn decode_file(data: &str) -> Result<Vec<u8>, ClientError> {
    if data.len() > MAX_FILE_BYTES.div_ceil(3) * 4 {
        return Err(protocol(
            "the attachment contents exceeded the pinned 64 MiB file limit",
        ));
    }
    let bytes = STANDARD
        .decode(data)
        .map_err(|_| protocol("the attachment contents were not valid base64"))?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(protocol(
            "the attachment contents exceeded the pinned 64 MiB file limit",
        ));
    }
    Ok(bytes)
}

fn require_success(success: bool) -> Result<(), ClientError> {
    if success {
        Ok(())
    } else {
        Err(protocol(
            "Wekan returned an unsuccessful attachment envelope without an error",
        ))
    }
}

fn decode_attachment<T: DeserializeOwned>(body: &[u8]) -> Result<T, ClientError> {
    let value = parse_json(body, "attachment")?;
    // Attachment successes contain a human message, also recognized by the
    // generic Meteor error decoder. Discriminate using the explicit success flag.
    if value.get("success") != Some(&serde_json::Value::Bool(true)) {
        if let Some(error) = embedded_error(&value, reqwest::StatusCode::OK)? {
            return Err(error);
        }
    }
    decode_value(value, "attachment", true)
}

impl WekanClient {
    fn attachment_endpoint(&self, segments: &[&str]) -> Result<reqwest::Url, ClientError> {
        let mut url = self
            .server()
            .join("api/attachment")
            .map_err(|_| protocol("could not build the attachment endpoint"))?;
        url.path_segments_mut()
            .map_err(|_| protocol("could not build the attachment endpoint"))?
            .extend(segments);
        Ok(url)
    }

    async fn attachment_get<T: DeserializeOwned>(
        &self,
        segments: &[&str],
        token: &SecretString,
        download: bool,
    ) -> Result<T, ClientError> {
        let request = self
            .http
            .get(self.attachment_endpoint(segments)?)
            .header(ACCEPT, "application/json")
            .bearer_auth(token.expose_secret());
        let body = if download {
            self.execute_success_request_with_limit(request, DOWNLOAD_RESPONSE_BYTES)
                .await?
        } else {
            self.execute_success_request(request).await?
        };
        decode_attachment(&body)
    }

    pub async fn attachments(
        &self,
        board: &str,
        swimlane: Option<&str>,
        list: Option<&str>,
        card: Option<&str>,
        token: &SecretString,
    ) -> Result<AttachmentListResponse, ClientError> {
        let mut segments = vec!["list", board];
        segments.extend(swimlane);
        segments.extend(list);
        segments.extend(card);
        let response: AttachmentListResponse = self.attachment_get(&segments, token, false).await?;
        require_success(response.success)?;
        if response.count as usize != response.attachments.len() {
            return Err(protocol(
                "the attachment count did not match the returned collection",
            ));
        }
        Ok(response)
    }

    pub async fn attachment_info(
        &self,
        id: &str,
        token: &SecretString,
    ) -> Result<AttachmentInfo, ClientError> {
        let response: AttachmentInfo = self.attachment_get(&["info", id], token, false).await?;
        require_success(response.success)?;
        if response.attachment_id != id {
            return Err(protocol(
                "the attachment response identifier did not match the request",
            ));
        }
        Ok(response)
    }

    pub async fn download_attachment(
        &self,
        id: &str,
        token: &SecretString,
    ) -> Result<AttachmentDownloadResponse, ClientError> {
        let response: AttachmentDownloadResponse =
            self.attachment_get(&["download", id], token, true).await?;
        require_success(response.success)?;
        if response.attachment_id != id {
            return Err(protocol(
                "the attachment response identifier did not match the request",
            ));
        }
        Ok(response)
    }

    pub async fn download_background(
        &self,
        board: &str,
        token: &SecretString,
    ) -> Result<BackgroundDownloadResponse, ClientError> {
        let response: BackgroundDownloadResponse = self
            .attachment_get(&["download-background", board], token, true)
            .await?;
        require_success(response.success)?;
        Ok(response)
    }

    async fn attachment_post<T: DeserializeOwned, R: Serialize>(
        &self,
        operation: &str,
        request: &R,
        token: &SecretString,
    ) -> Result<T, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.attachment_endpoint(&[operation])?)
                    .timeout(POST_TIMEOUT)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        decode_attachment(&body)
    }

    pub async fn upload_attachment(
        &self,
        request: &UploadAttachmentRequest,
        token: &SecretString,
    ) -> Result<AttachmentUploadResponse, ClientError> {
        let response: AttachmentUploadResponse =
            self.attachment_post("upload", request, token).await?;
        require_success(response.success)?;
        Ok(response)
    }

    pub async fn upload_background(
        &self,
        request: &UploadBackgroundRequest,
        token: &SecretString,
    ) -> Result<BackgroundUploadResponse, ClientError> {
        let response: BackgroundUploadResponse = self
            .attachment_post("upload-background", request, token)
            .await?;
        require_success(response.success)?;
        Ok(response)
    }

    pub async fn copy_attachment(
        &self,
        request: &TransferAttachmentRequest,
        token: &SecretString,
    ) -> Result<AttachmentCopyResponse, ClientError> {
        let response: AttachmentCopyResponse = self.attachment_post("copy", request, token).await?;
        require_success(response.success)?;
        if response.source_attachment_id != request.attachment_id {
            return Err(protocol(
                "the copy response source identifier did not match the request",
            ));
        }
        Ok(response)
    }

    pub async fn move_attachment(
        &self,
        request: &TransferAttachmentRequest,
        token: &SecretString,
    ) -> Result<AttachmentMoveResponse, ClientError> {
        let response: AttachmentMoveResponse = self.attachment_post("move", request, token).await?;
        require_success(response.success)?;
        if response.attachment_id != request.attachment_id
            || response.target_board_id != request.target_board_id
        {
            return Err(protocol(
                "the move response identifiers did not match the request",
            ));
        }
        Ok(response)
    }

    pub async fn delete_attachment(
        &self,
        id: &str,
        token: &SecretString,
    ) -> Result<AttachmentDeleteResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .request(Method::DELETE, self.attachment_endpoint(&["delete", id])?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: AttachmentDeleteResponse = decode_attachment(&body)?;
        require_success(response.success)?;
        if response.attachment_id != id {
            return Err(protocol(
                "the deletion response identifier did not match the request",
            ));
        }
        Ok(response)
    }
}

impl WekanClient {
    pub async fn export_attachment(
        &self,
        board: &str,
        attachment: &str,
        options: super::exports::ExportOptions<'_>,
        file: &mut std::fs::File,
    ) -> Result<super::exports::ExportMetadata, super::exports::ExportError> {
        if options.format != super::exports::ExportFormat::Json {
            return Err(super::exports::protocol("attachment exports require json").into());
        }
        self.export_resource(
            &[board, "attachments", attachment, "export"],
            options,
            file,
            true,
        )
        .await
    }
}
