//! Shared stored metadata returned by native v13.28 resource reads.
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Number;
use std::collections::BTreeMap;
fn required_nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}
/// Wekan v13.28: models/lib/scrum.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ScrumSettings {
    pub enabled: Option<bool>,
    pub product_goal: Option<String>,
    pub definition_of_done: Option<String>,
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub definition_of_ready: Option<String>,
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub standup_timer: Option<bool>,
    #[serde(default, deserialize_with = "standup_minutes")]
    pub standup_minutes: Option<u8>,
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub retrospective_board: Option<bool>,
    pub estimate_source: Option<String>,
    pub estimate_custom_field_id: Option<String>,
    pub estimate_unit: Option<String>,
    pub completion_policy: Option<String>,
    pub product_owner_id: Option<String>,
    pub scrum_master_id: Option<String>,
    pub developer_ids: Option<Vec<String>>,
    pub working_days: Option<Vec<u8>>,
    pub visibility: Option<ScrumVisibility>,
}

fn standup_minutes<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u8>, D::Error> {
    let minutes = u8::deserialize(d)?;
    if !(1..=15).contains(&minutes) {
        return Err(serde::de::Error::custom(
            "standup duration must be 1 to 15 minutes",
        ));
    }
    Ok(Some(minutes))
}
/// Wekan v13.28: models/lib/scrum.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ScrumVisibility {
    pub card_sprint: Option<bool>,
    pub card_past_sprints: Option<bool>,
    pub card_release: Option<bool>,
    pub card_issue_type: Option<bool>,
    pub card_acceptance_criteria: Option<bool>,
    pub card_backlog_rank: Option<bool>,
    pub minicard_sprint: Option<bool>,
    pub minicard_past_sprints: Option<bool>,
    pub minicard_release: Option<bool>,
    pub minicard_issue_type: Option<bool>,
    pub minicard_acceptance_criteria: Option<bool>,
    pub minicard_backlog_rank: Option<bool>,
    pub list_category: Option<bool>,
    pub swimlane_sprint: Option<bool>,
    pub swimlane_release: Option<bool>,
    pub swimlane_purpose: Option<bool>,
}
/// Wekan v13.28: models/lib/scrum.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ScrumCard {
    pub sprint_id: Option<String>,
    pub past_sprint_ids: Option<Vec<String>>,
    pub backlog_rank: Option<Number>,
    pub release_id: Option<String>,
    pub release_ids: Option<Vec<String>>,
    pub issue_type: Option<String>,
    pub acceptance_criteria: Option<String>,
}
/// Wekan v13.28: models/lib/scrum.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ScrumList {
    pub category: Option<String>,
}
/// Wekan v13.28: models/lib/scrum.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ScrumSwimlane {
    pub sprint_id: Option<String>,
    pub release_id: Option<String>,
    pub purpose: Option<String>,
}
/// Wekan v13.28: models/lib/scrumTransfer.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ScrumImportLoss {
    pub path: String,
    pub source_id: String,
    pub reason: String,
}
/// Wekan v13.28: models/boards.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardAnnouncement {
    pub enabled: Option<bool>,
    pub body: Option<String>,
    pub updated_at: Option<String>,
}
/// Wekan v13.28: models/boards.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardWipGroup {
    #[serde(rename(deserialize = "_id", serialize = "id"))]
    pub id: String,
    pub name: Option<String>,
    pub list_ids: Vec<String>,
    pub limit: Number,
    pub enabled: bool,
}
/// Wekan v13.28: models/lib/columnSets.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardColumnSet {
    #[serde(rename(deserialize = "_id", serialize = "id"))]
    pub id: String,
    pub name: String,
    pub columns: Vec<BoardColumn>,
}
/// Wekan v13.28: models/lib/columnSets.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardColumn {
    #[serde(rename(deserialize = "_id", serialize = "id"))]
    pub id: String,
    pub title: String,
    pub color: String,
}
/// Wekan v13.28: models/lib/emailToCard.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct EmailParts {
    pub description: Option<String>,
    pub strip_quoted: Option<bool>,
    pub attachments: Option<bool>,
    pub sender_line: Option<bool>,
}
/// Wekan v13.28: models/lib/emailToCard.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardEmailToCard {
    pub enabled: Option<bool>,
    pub list_id: Option<String>,
    pub swimlane_id: Option<String>,
    pub parts: Option<EmailParts>,
}
/// Wekan v13.28: models/lib/emailToCard.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct MemberEmailToCard {
    pub enabled: Option<bool>,
    pub parts: Option<EmailParts>,
}
/// Wekan v13.28: models/lib/notificationDelivery.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct NotificationDelivery {
    pub email: Option<NotificationChannel>,
    pub tray: Option<NotificationChannel>,
    pub webhook: Option<WebhookDelivery>,
}
/// Wekan v13.28: models/lib/notificationDelivery.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct MemberNotificationDelivery {
    pub email: Option<NotificationChannel>,
    pub tray: Option<NotificationChannel>,
}
/// Wekan v13.28: models/lib/notificationDelivery.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct NotificationChannel {
    pub layout: Option<String>,
    pub parts: Option<Vec<String>>,
    pub grouping: Option<String>,
    pub schedule: Option<String>,
    pub interval_minutes: Option<Number>,
    pub daily_time: Option<String>,
    pub quiet_start: Option<String>,
    pub quiet_end: Option<String>,
    pub timezone: Option<String>,
}
/// Wekan v13.28: models/lib/notificationDelivery.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct WebhookDelivery {
    pub text: Option<bool>,
    pub fields: Option<Vec<String>>,
    pub grouping: Option<String>,
    pub schedule: Option<String>,
    pub interval_minutes: Option<Number>,
    pub daily_time: Option<String>,
    pub quiet_start: Option<String>,
    pub quiet_end: Option<String>,
    pub timezone: Option<String>,
}
/// Wekan v13.28: models/lib/chatView.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ChatViewLayout {
    pub tree_width: Number,
    pub chat_width: Number,
    pub tree_collapsed: bool,
    pub card_collapsed: bool,
    pub chat_collapsed: bool,
}
/// Wekan v13.28: server/models/dependencies.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct PersonalDependency {
    pub board_id: String,
    pub card_id: String,
    pub target_card_id: String,
    pub r#type: String,
    pub color: Option<String>,
    pub icon: Option<String>,
}
/// Wekan v13.28: models/cards.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardCustomFieldLink {
    pub card_id: String,
    pub mode: String,
    pub user_id: String,
    pub created_at: Option<String>,
}
/// Wekan v13.28: server/emailToCard.js, server/emailView.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardEmail {
    pub direction: String,
    #[serde(rename(deserialize = "from", serialize = "sender"))]
    #[serde(deserialize_with = "required_nullable")]
    pub sender: Option<EmailSender>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub subject: String,
    #[serde(deserialize_with = "required_nullable")]
    pub message_id: Option<String>,
    pub thread_ids: Vec<String>,
    pub at: String,
}
/// Wekan v13.28: server/emailToCard.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct EmailSender {
    pub name: String,
    pub address: String,
}
/// Wekan v13.28: models/lib/cardDrawing.js, server/methods/whiteboard.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardDrawing {
    #[serde(rename(deserialize = "_id", serialize = "id"))]
    pub id: String,
    pub strokes: Vec<DrawingStroke>,
    pub width: Number,
    pub height: Number,
    pub png_attachment_id: Option<String>,
    pub updated_at: Option<String>,
}
/// Wekan v13.28: models/lib/cardDrawing.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct DrawingStroke {
    #[serde(rename(deserialize = "_id", serialize = "id"))]
    pub id: Option<String>,
    pub user_id: Option<String>,
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub group: Option<String>,
    pub color: String,
    pub width: Number,
    pub points: Vec<[Number; 2]>,
}
/// Wekan v13.28: models/lib/listPermissions.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ListRoleException {
    pub write: Option<bool>,
    pub comment: Option<bool>,
}
/// Wekan v13.28: models/lists.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ListSyncSource {
    pub r#type: Option<String>,
    pub url: Option<String>,
    pub project_key: Option<String>,
    pub time_mapping_identities: Option<BTreeMap<String, String>>,
    pub estimate_custom_field_id: Option<String>,
    pub estimate_mapping_identity: Option<String>,
    pub estimate_source_field: Option<String>,
    pub fields: Option<Vec<String>>,
    pub create_cards: Option<bool>,
    pub archive_cards: Option<bool>,
    pub issue_details: Option<bool>,
    pub write_back: Option<bool>,
    pub write_back_since: Option<String>,
    pub enabled: Option<bool>,
    pub last_synced_at: Option<String>,
    pub last_sync_error: Option<String>,
}
/// Wekan v13.28: models/cards.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardSyncLastSource {
    pub title: Option<String>,
    pub description: Option<String>,
    pub estimate: Option<Number>,
    pub original_estimate: Option<Number>,
    pub remaining_estimate: Option<Number>,
    pub original_estimate_mapping: Option<String>,
    pub remaining_estimate_mapping: Option<String>,
    pub estimate_mapping: Option<String>,
    pub spent_time: Option<Number>,
    pub sprint: Option<String>,
    pub releases: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardFlow {
    pub blocked: Option<bool>,
    pub blocked_reason: Option<String>,
    pub blocked_at: Option<String>,
    pub blocked_by: Option<String>,
    pub class_of_service: Option<String>,
    pub moscow: Option<String>,
    pub wsjf: Option<Wsjf>,
    pub split_done_list_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct Wsjf {
    pub business_value: Option<u8>,
    pub time_criticality: Option<u8>,
    pub risk_reduction: Option<u8>,
    pub job_size: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ShopSettings {
    pub enabled: bool,
    pub title: String,
    pub intro: String,
    pub currency: String,
    pub price_field_id: String,
    pub sku_field_id: String,
    pub stock_field_id: String,
    pub sale_list_ids: Vec<String>,
    pub category_label_ids: Vec<String>,
    pub orders_board_id: String,
    pub orders_list_id: String,
    pub guest_checkout: bool,
    pub terms_url: String,
    pub vat_percent: Number,
    pub prices_include_vat: bool,
    pub reference_type: String,
    pub decrement_stock: bool,
    pub confirmation_email: bool,
    pub payment_methods: Vec<ShopPaymentMethod>,
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub updated_by: Option<String>,
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub updated_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ShopPaymentMethod {
    pub id: String,
    pub enabled: bool,
    pub method: PaymentMethod,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", deny_unknown_fields, rename_all = "kebab-case")]
pub enum PaymentMethod {
    Sepa {
        name: String,
        iban: String,
        bic: String,
    },
    FiBank {
        name: String,
        iban: String,
        bic: String,
    },
    SwissQr {
        name: String,
        iban: String,
        street: String,
        building: String,
        postcode: String,
        town: String,
        country: String,
    },
    BankTransfer {
        name: String,
        details: String,
    },
    Swish {
        payee: String,
    },
    Pix {
        key: String,
        name: String,
        city: String,
    },
    Upi {
        vpa: String,
        name: String,
    },
    Bitcoin {
        address: String,
    },
    Paypal {
        user: String,
    },
    Link {
        label: String,
        url: String,
    },
    Promptpay {
        target: String,
    },
    Paynow {
        proxy: String,
        name: String,
    },
    Vietqr {
        bin: String,
        account: String,
    },
    Fps {
        proxy: String,
        name: String,
    },
    Spayd {
        iban: String,
        bic: String,
    },
    Zbp {
        account: String,
        name: String,
        nip: String,
    },
    Mnb {
        name: String,
        iban: String,
        bic: String,
    },
    Ips {
        account: String,
        name: String,
    },
    DkFik {
        creditor: String,
    },
    Ethereum {
        address: String,
    },
    Litecoin {
        address: String,
    },
    Cash {
        details: String,
    },
    Cheque {
        details: String,
    },
    Other {
        label: String,
        details: String,
    },
}
