use crate::client::metadata::*;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ListArchiveSuccess {
    pub board_id: String,
    pub list_id: String,
    pub archived: bool,
}
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
    pub scrum: Option<ScrumList>,
    pub role_exceptions: Option<std::collections::BTreeMap<String, ListRoleException>>,
    pub sync_source: Option<Box<ListSyncSource>>,

    pub scrum_revision: Option<serde_json::Number>,
    pub cards_use_list_color: Option<bool>,
    pub linked_board_id: Option<String>,
    pub linked_list_id: Option<String>,
    pub sync_revision: Option<String>,
    pub sync_credential_generation: Option<serde_json::Number>,
    pub sync_credential_fence: Option<String>,
    pub sync_write_back_error: Option<String>,
    pub sync_credential_incarnation: Option<String>,
    pub list_id: String,
    pub title: String,
    pub watchers: Option<Vec<String>>,
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
    pub deletion_requested: bool,
    pub delete_mode: ListDeleteMode,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ListTransferSuccess {
    pub source_board_id: String,
    pub source_list_id: String,
    pub target_board_id: String,
    pub target_swimlane_id: Option<String>,
    pub returned_list_id: String,
}
