use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntegrationDetail {
    pub integration_id: String,
    pub enabled: bool,
    pub title: Option<String>,
    pub integration_type: String,
    pub activities: Vec<String>,
    pub url: String,
    pub board_id: String,
    pub created_at: String,
    pub modified_at: String,
    pub user_id: String,
}

impl From<crate::client::integrations::IntegrationDocument> for IntegrationDetail {
    fn from(d: crate::client::integrations::IntegrationDocument) -> Self {
        Self {
            integration_id: d.integration_id,
            enabled: d.enabled,
            title: d.title,
            integration_type: d.integration_type,
            activities: d.activities,
            url: d.url,
            board_id: d.board_id,
            created_at: d.created_at,
            modified_at: d.modified_at,
            user_id: d.user_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntegrationCollectionSuccess {
    pub board_id: String,
    pub integrations: Vec<IntegrationDetail>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntegrationCreateSuccess {
    pub board_id: String,
    pub integration_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationSubmittedField {
    Enabled,
    Title,
    Url,
    Token,
    Activities,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntegrationUpdateSuccess {
    pub board_id: String,
    pub integration_id: String,
    pub submitted_fields: Vec<IntegrationSubmittedField>,
    pub mutation_confirmed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntegrationDeleteSuccess {
    pub board_id: String,
    pub integration_id: String,
    pub deletion_requested: bool,
    pub mutation_confirmed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationActivityAction {
    Add,
    Remove,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntegrationActivityMutationSuccess {
    pub board_id: String,
    pub integration_id: String,
    pub action: IntegrationActivityAction,
    pub requested_activities: Vec<String>,
    pub activities: Vec<String>,
}
