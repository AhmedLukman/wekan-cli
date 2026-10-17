use crate::client::attachments::AttachmentMetadata;
use serde::Serialize;

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
pub struct AttachmentMutationSuccess<T> {
    #[serde(flatten)]
    pub response: T,
    pub mutation_confirmed: bool,
}

impl<T> AttachmentMutationSuccess<T> {
    pub fn acknowledged(response: T) -> Self {
        Self {
            response,
            mutation_confirmed: false,
        }
    }
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
