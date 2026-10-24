use serde::Serialize;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct InboundEmailReceipt {
    pub resource_id: Option<String>,
}

impl From<crate::client::integrations::inbound_email::InboundEmailReceipt> for InboundEmailReceipt {
    fn from(receipt: crate::client::integrations::inbound_email::InboundEmailReceipt) -> Self {
        Self {
            resource_id: receipt.resource_id,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BounceReceipt {
    pub ok: bool,
    pub recorded: u64,
}

impl From<crate::client::integrations::inbound_email::BounceReceipt> for BounceReceipt {
    fn from(receipt: crate::client::integrations::inbound_email::BounceReceipt) -> Self {
        Self {
            ok: receipt.ok,
            recorded: receipt.recorded,
        }
    }
}

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
    pub notification_delivery: Option<crate::client::integrations::WebhookNotificationDelivery>,
    pub mattermost_direct_messages: Option<bool>,
    pub bot: Option<bool>,
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
            notification_delivery: d.notification_delivery,
            mattermost_direct_messages: d.mattermost_direct_messages,
            bot: d.bot,
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
    NotificationDelivery,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntegrationUpdateSuccess {
    pub board_id: String,
    pub integration_id: String,
    pub submitted_fields: Vec<IntegrationSubmittedField>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IntegrationDeleteSuccess {
    pub board_id: String,
    pub integration_id: String,
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
