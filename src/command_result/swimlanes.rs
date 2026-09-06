use serde::Serialize;
use serde_json::Number;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneSummary {
    pub swimlane_id: String,
    pub title: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneCollectionSuccess {
    pub board_id: String,
    pub swimlanes: Vec<SwimlaneSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneDetail {
    pub swimlane_id: String,
    pub title: String,
    pub archived: bool,
    pub archived_at: Option<String>,
    pub board_id: String,
    pub created_at: String,
    pub sort: Option<Number>,
    pub color: Option<String>,
    pub updated_at: Option<String>,
    pub modified_at: String,
    pub swimlane_type: String,
    pub height: Option<Number>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneCreateSuccess {
    pub board_id: String,
    pub swimlane_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SwimlaneUpdatedField {
    Title,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneUpdateSuccess {
    pub board_id: String,
    pub swimlane_id: String,
    pub updated_fields: Vec<SwimlaneUpdatedField>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SwimlaneDeleteMode {
    Hard,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneDeleteSuccess {
    pub board_id: String,
    pub swimlane_id: String,
    pub deleted: bool,
    pub delete_mode: SwimlaneDeleteMode,
}
