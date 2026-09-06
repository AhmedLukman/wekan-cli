use serde::Serialize;
use serde_json::Number;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardSummary {
    pub card_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub swimlane_id: Option<String>,
    pub received_at: Option<String>,
    pub start_at: Option<String>,
    pub due_at: Option<String>,
    pub end_at: Option<String>,
    pub assignees: Vec<String>,
    pub sort: Option<Number>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CardCollectionSuccess {
    pub board_id: String,
    pub list_id: String,
    pub cards: Vec<CardSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CardDetailCustomFieldValue {
    String(String),
    Number(Number),
    Boolean(bool),
    Strings(Vec<String>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailCustomField {
    pub custom_field_id: Option<String>,
    pub value: Option<CardDetailCustomFieldValue>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CardDetailStickerHighlight {
    Underline,
    Round,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailSticker {
    pub icon: Option<String>,
    pub name: Option<String>,
    pub highlight: Option<CardDetailStickerHighlight>,
    pub position: Option<Number>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailLocation {
    pub location_id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<Number>,
    pub longitude: Option<Number>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailDependency {
    pub card_id: String,
    pub dependency_type: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailVote {
    pub question: String,
    pub positive: Vec<String>,
    pub negative: Vec<String>,
    pub end: Option<String>,
    pub public: bool,
    pub allow_non_board_members: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailPoker {
    pub question: Option<bool>,
    pub one: Vec<String>,
    pub two: Vec<String>,
    pub three: Vec<String>,
    pub five: Vec<String>,
    pub eight: Vec<String>,
    pub thirteen: Vec<String>,
    pub twenty: Vec<String>,
    pub forty: Vec<String>,
    pub one_hundred: Vec<String>,
    pub unsure: Vec<String>,
    pub end: Option<String>,
    pub allow_non_board_members: Option<bool>,
    pub estimation: Option<Number>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetail {
    pub card_id: String,
    pub title: Option<String>,
    pub archived: bool,
    pub archived_at: Option<String>,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
    pub delete_batch_id: Option<String>,
    pub parent_id: Option<String>,
    pub list_id: Option<String>,
    pub swimlane_id: String,
    pub board_id: Option<String>,
    pub cover_id: Option<String>,
    pub color: Option<String>,
    pub created_at: String,
    pub modified_at: String,
    pub custom_fields: Vec<CardDetailCustomField>,
    pub date_last_activity: String,
    pub description: Option<String>,
    pub requested_by: Option<String>,
    pub assigned_by: Option<String>,
    pub label_ids: Vec<String>,
    pub members: Vec<String>,
    pub assignees: Vec<String>,
    pub requesters: Vec<String>,
    pub assigners: Vec<String>,
    pub received_at: Option<String>,
    pub start_at: Option<String>,
    pub due_at: Option<String>,
    pub end_at: Option<String>,
    pub due_complete: Option<bool>,
    pub stickers: Vec<CardDetailSticker>,
    pub location_name: Option<String>,
    pub location_address: Option<String>,
    pub location_latitude: Option<Number>,
    pub location_longitude: Option<Number>,
    pub locations: Vec<CardDetailLocation>,
    pub spent_time: Option<Number>,
    pub is_overtime: Option<bool>,
    pub user_id: String,
    pub sort: Option<Number>,
    pub subtask_sort: Option<Number>,
    pub card_type: String,
    pub linked_id: Option<String>,
    pub card_dependencies: Vec<CardDetailDependency>,
    pub vote: Option<CardDetailVote>,
    pub poker: Option<CardDetailPoker>,
    pub target_id_gantt: Vec<String>,
    pub link_type_gantt: Vec<Number>,
    pub link_id_gantt: Vec<String>,
    pub card_number: Option<Number>,
    pub show_activities: bool,
    pub show_list_on_minicard: Option<bool>,
    pub show_checklist_at_minicard: Option<bool>,
    pub hide_finished_checklist_if_items_are_hidden: Option<bool>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CardCreateSuccess {
    pub board_id: String,
    pub list_id: String,
    pub card_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardSubmittedField {
    Title,
    Sort,
    ParentId,
    Description,
    Color,
    LabelIds,
    RequestedBy,
    AssignedBy,
    ReceivedAt,
    StartAt,
    DueAt,
    EndAt,
    SpentTime,
    IsOverTime,
    Members,
    Assignees,
    DueComplete,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CardUpdateSuccess {
    pub board_id: String,
    pub list_id: String,
    pub card_id: String,
    pub submitted_fields: Vec<CardSubmittedField>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardDeleteMode {
    Hard,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CardDeleteSuccess {
    pub board_id: String,
    pub list_id: String,
    pub card_id: String,
    pub deleted: bool,
    pub delete_mode: CardDeleteMode,
}
