use crate::client::boards::import::ZipImportCounts;
use serde::Serialize;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BoardImportSuccess {
    pub board_id: String,
    pub source: String,
    pub mutation_confirmed: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ZipImportSuccess {
    pub board_id: String,
    pub swimlane_id: Option<String>,
    pub list_id: Option<String>,
    pub card_id: Option<String>,
    pub checklist_id: Option<String>,
    pub counts: ZipImportCounts,
    pub mutation_confirmed: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct IcsImportSuccess {
    pub board_id: String,
    pub swimlane_id: String,
    pub list_id: String,
    pub created: u64,
    pub card_ids: Vec<String>,
    pub mutation_confirmed: bool,
}
