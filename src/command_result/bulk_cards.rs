use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BulkCardCreateItem {
    pub index: usize,
    pub card_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BulkCardCreateResult {
    pub board_id: String,
    pub list_id: String,
    pub results: Vec<BulkCardCreateItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BulkCardDeleteResult {
    pub board_id: String,
    pub deleted: Vec<String>,
    pub not_found: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BulkCardLabelUpdate {
    pub card_id: String,
    pub label_ids: Vec<Option<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BulkCardLabelsResult {
    pub board_id: String,
    pub updated: Vec<BulkCardLabelUpdate>,
    pub not_found: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "operation", content = "data", rename_all = "snake_case")]
pub enum CardBatchResult {
    CardBulkCreate(BulkCardCreateResult),
    CardBulkDelete(BulkCardDeleteResult),
    CardBulkLabels(BulkCardLabelsResult),
}

impl CardBatchResult {
    pub fn incomplete(&self) -> bool {
        match self {
            Self::CardBulkCreate(data) => data.results.iter().any(|item| item.error.is_some()),
            Self::CardBulkDelete(data) => !data.not_found.is_empty(),
            Self::CardBulkLabels(data) => !data.not_found.is_empty(),
        }
    }
}
