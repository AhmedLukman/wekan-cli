use serde::Serialize;
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustomFieldItemCreateSuccess {
    pub board_id: String,
    pub custom_field_id: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustomFieldItemUpdateSuccess {
    pub board_id: String,
    pub custom_field_id: String,
    pub item_id: String,
    pub submitted_fields: Vec<CustomFieldItemSubmittedField>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustomFieldItemDeleteSuccess {
    pub board_id: String,
    pub custom_field_id: String,
    pub item_id: String,
    pub delete_mode: CustomFieldItemDeleteMode,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomFieldItemSubmittedField {
    Name,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomFieldItemDeleteMode {
    Hard,
}
