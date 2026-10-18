use serde::Serialize;
use serde_json::Number;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependencySummary {
    pub target_card_id: String,
    pub dependency_type: String,
    pub color: String,
    pub icon: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependencyLine {
    pub source_card_id: String,
    pub source_title: Option<String>,
    pub source_card_number: Option<Number>,
    pub target_card_id: String,
    pub target_title: Option<String>,
    pub target_card_number: Option<Number>,
    pub dependency_type: String,
    pub color: String,
    pub icon: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDependencyCollectionSuccess {
    pub board_id: String,
    pub card_id: String,
    pub dependencies: Vec<DependencySummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BoardDependencyCollectionSuccess {
    pub board_id: String,
    pub dependencies: Vec<DependencyLine>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependencyCreateSuccess {
    pub board_id: String,
    pub card_id: String,
    pub target_card_id: String,
    pub dependency_type: String,
    pub color: String,
    pub icon: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencySubmittedField {
    Type,
    Color,
    Icon,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependencyUpdateSuccess {
    pub board_id: String,
    pub card_id: String,
    pub target_card_id: String,
    pub submitted_fields: Vec<DependencySubmittedField>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DependencyDeleteSuccess {
    pub board_id: String,
    pub card_id: String,
    pub target_card_id: String,
}
