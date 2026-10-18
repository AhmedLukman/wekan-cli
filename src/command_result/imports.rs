use crate::client::boards::import::ZipImportCounts;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TrelloImportResult {
    pub format: crate::client::boards::trello_import::TrelloImportFormat,
    pub board_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<crate::client::boards::trello_import::TrelloImportBoardError>>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BoardImportSuccess {
    pub board_id: String,
    pub source: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ZipImportSuccess {
    pub board_id: String,
    pub swimlane_id: Option<String>,
    pub list_id: Option<String>,
    pub card_id: Option<String>,
    pub checklist_id: Option<String>,
    pub counts: ZipImportCounts,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct IcsImportSuccess {
    pub board_id: String,
    pub swimlane_id: String,
    pub list_id: String,
    pub created: u64,
    pub card_ids: Vec<String>,
}
