use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustomFieldSummary {
    pub custom_field_id: String,
    pub name: String,
    pub field_type: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustomFieldDropdownItem {
    pub item_id: String,
    pub name: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustomFieldSettings {
    pub currency_code: Option<String>,
    pub dropdown_items: Option<Vec<CustomFieldDropdownItem>>,
    pub stringtemplate_format: Option<String>,
    pub stringtemplate_separator: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustomFieldDetail {
    pub custom_field_id: String,
    pub board_ids: Vec<String>,
    pub name: String,
    pub field_type: String,
    pub settings: CustomFieldSettings,
    pub show_on_card: bool,
    pub automatically_on_card: bool,
    pub always_on_card: bool,
    pub show_label_on_mini_card: bool,
    pub show_sum_at_top_of_list: bool,
    pub created_at: Option<String>,
    pub modified_at: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustomFieldCollectionSuccess {
    pub board_id: String,
    pub custom_fields: Vec<CustomFieldSummary>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustomFieldCreateSuccess {
    pub board_id: String,
    pub custom_field_id: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustomFieldUpdateSuccess {
    pub board_id: String,
    pub custom_field_id: String,
    pub submitted_fields: Vec<CustomFieldSubmittedField>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CustomFieldDeleteSuccess {
    pub board_id: String,
    pub custom_field_id: String,
    pub delete_mode: CustomFieldDeleteMode,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomFieldSubmittedField {
    Name,
    Type,
    Settings,
    ShowOnCard,
    AutomaticallyOnCard,
    AlwaysOnCard,
    ShowLabelOnMiniCard,
    ShowSumAtTopOfList,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomFieldDeleteMode {
    Hard,
}
