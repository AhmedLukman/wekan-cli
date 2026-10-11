use serde::Serialize;
use serde_json::Number;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChecklistItemDetail {
    pub item_id: String,
    pub title: String,
    pub sort: Number,
    pub is_finished: bool,
    pub checklist_id: String,
    pub card_id: String,
    pub board_id: Option<String>,
    pub user_id: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChecklistItemCreateSuccess {
    pub board_id: String,
    pub card_id: String,
    pub checklist_id: String,
    pub item_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChecklistItemUpdateSuccess {
    pub board_id: String,
    pub card_id: String,
    pub checklist_id: String,
    pub item_id: String,
    pub submitted_fields: Vec<ChecklistItemSubmittedField>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChecklistItemDeleteSuccess {
    pub board_id: String,
    pub card_id: String,
    pub checklist_id: String,
    pub item_id: String,
    pub deleted: bool,
    pub delete_mode: ChecklistItemDeleteMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistItemDeleteMode {
    Hard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistItemSubmittedField {
    Title,
    IsFinished,
}
