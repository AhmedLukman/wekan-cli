use serde::Serialize;
use serde_json::Number;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ListSummary {
    pub list_id: String,
    pub title: String,
    pub modified_at: Option<String>,
    pub cards_modified_at: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ListCollectionSuccess {
    pub board_id: String,
    pub lists: Vec<ListSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ListWipLimitDetail {
    pub value: Number,
    pub enabled: bool,
    pub soft: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ListDetail {
    pub list_id: String,
    pub title: String,
    pub starred: Option<bool>,
    pub archived: bool,
    pub archived_at: Option<String>,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
    pub delete_batch_id: Option<String>,
    pub board_id: String,
    pub swimlane_id: Option<String>,
    pub created_at: String,
    pub sort: Option<Number>,
    pub updated_at: Option<String>,
    pub modified_at: String,
    pub position_updated_at: Option<String>,
    pub wip_limit: Option<ListWipLimitDetail>,
    pub color: Option<String>,
    pub list_type: String,
    pub width: Option<Number>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ListCreateSuccess {
    pub board_id: String,
    pub list_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ListUpdatedField {
    Title,
    Color,
    Starred,
    WipLimit,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ListUpdateSuccess {
    pub board_id: String,
    pub list_id: String,
    pub updated_fields: Vec<ListUpdatedField>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ListDeleteMode {
    Soft,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ListDeleteSuccess {
    pub board_id: String,
    pub list_id: String,
    pub deleted: bool,
    pub delete_mode: ListDeleteMode,
}
