use crate::client::exports::{ExportFormat, ExportMetadata, ExportTarget};
use serde::Serialize;

#[derive(Debug)]
pub(crate) struct StagedExport(pub(crate) tempfile::NamedTempFile);
impl PartialEq for StagedExport {
    fn eq(&self, other: &Self) -> bool {
        self.0.path() == other.0.path()
    }
}
impl Eq for StagedExport {}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ExportSuccess {
    pub resource: ExportResource,
    pub board_id: Option<String>,
    pub chart_key: Option<String>,
    pub list_id: Option<String>,
    pub card_id: Option<String>,
    pub attachment_id: Option<String>,
    pub format: ExportFormat,
    pub destination: String,
    pub bytes_written: u64,
    pub file_saved: bool,
    pub http_status: u16,
    pub content_type: Option<String>,
    #[serde(skip)]
    pub(crate) staged: Option<StagedExport>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportResource {
    Board,
    Boards,
    Chart,
    Card,
    Attachment,
}
impl ExportSuccess {
    pub(crate) fn new(
        target: ExportTarget,
        format: ExportFormat,
        destination: String,
        response: ExportMetadata,
    ) -> Self {
        let (resource, board_id, list_id, card_id, attachment_id, chart_key) = match target {
            ExportTarget::AllBoards => (ExportResource::Boards, None, None, None, None, None),
            ExportTarget::Chart {
                board_id,
                chart_key,
            } => (
                ExportResource::Chart,
                Some(board_id),
                None,
                None,
                None,
                Some(chart_key),
            ),
            ExportTarget::Board { board_id } => (
                ExportResource::Board,
                Some(board_id),
                None,
                None,
                None,
                None,
            ),
            ExportTarget::Card {
                board_id,
                list_id,
                card_id,
            } => (
                ExportResource::Card,
                Some(board_id),
                Some(list_id),
                Some(card_id),
                None,
                None,
            ),
            ExportTarget::Attachment {
                board_id,
                attachment_id,
            } => (
                ExportResource::Attachment,
                Some(board_id),
                None,
                None,
                Some(attachment_id),
                None,
            ),
        };
        Self {
            resource,
            board_id,
            chart_key,
            list_id,
            card_id,
            attachment_id,
            format,
            destination,
            bytes_written: response.bytes_written,
            file_saved: false,
            http_status: response.http_status,
            content_type: response.content_type,
            staged: None,
        }
    }
}
