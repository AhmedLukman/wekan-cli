use serde::Serialize;
use serde_json::Number;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChecklistSummary {
    pub checklist_id: String,
    pub title: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChecklistItemSummary {
    pub item_id: String,
    pub title: String,
    pub is_finished: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChecklistDetail {
    pub checklist_id: String,
    pub card_id: String,
    pub board_id: Option<String>,
    pub user_id: Option<String>,
    pub title: String,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub modified_at: String,
    pub sort: Number,
    pub hide_checked_checklist_items: Option<bool>,
    pub hide_all_checklist_items: Option<bool>,
    pub show_checklist_at_minicard: Option<bool>,
    pub items: Vec<ChecklistItemSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChecklistCollectionSuccess {
    pub board_id: String,
    pub card_id: String,
    pub checklists: Vec<ChecklistSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChecklistCreateSuccess {
    pub board_id: String,
    pub card_id: String,
    pub checklist_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChecklistDeleteSuccess {
    pub board_id: String,
    pub card_id: String,
    pub checklist_id: String,
    pub deleted: bool,
    pub delete_mode: ChecklistDeleteMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistDeleteMode {
    Hard,
}
