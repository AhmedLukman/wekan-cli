use crate::client::attachments::{AttachmentMetadata, AttachmentUploadResponse};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AttachmentUploadSuccess {
    pub attachment_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub storage_backend: String,
    pub server_message: String,
    pub request_accepted: bool,
    pub upload_confirmed: bool,
}

impl From<AttachmentUploadResponse> for AttachmentUploadSuccess {
    fn from(response: AttachmentUploadResponse) -> Self {
        Self {
            attachment_id: response.attachment_id,
            file_name: response.file_name,
            file_size: response.file_size,
            storage_backend: response.storage_backend,
            server_message: response.message,
            request_accepted: true,
            // v12.57 validates and migrates the attachment in deferred tasks.
            upload_confirmed: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DeletedAttachmentCollectionSuccess {
    pub board_id: String,
    pub count: usize,
    pub attachments: Vec<crate::client::attachments::recovery::DeletedAttachment>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AttachmentRestoreSuccess {
    pub board_id: String,
    pub attachment_id: String,
    pub restored: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AttachmentSoftDeleteSuccess {
    pub board_id: String,
    pub attachment_id: String,
    pub deleted: bool,
    pub batch_id: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BoardAttachmentCollectionSuccess {
    pub board_id: String,
    pub count: usize,
    pub attachments: Vec<crate::client::boards::attachments::BoardAttachment>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentScope {
    Board,
    Swimlane,
    List,
    Card,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AttachmentCollectionSuccess {
    pub board_id: String,
    pub swimlane_id: Option<String>,
    pub list_id: Option<String>,
    pub card_id: Option<String>,
    pub requested_scope: AttachmentScope,
    pub effective_scope: AttachmentScope,
    pub count: u32,
    pub attachments: Vec<AttachmentMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AttachmentDownloadSuccess {
    pub attachment_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub file_type: String,
    pub storage_backend: String,
    pub background_image_url: Option<String>,
    pub destination: String,
    pub bytes_written: u64,
    pub file_saved: bool,
    #[serde(skip)]
    pub bytes: Option<Vec<u8>>,
}
