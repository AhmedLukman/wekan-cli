use crate::client::rules::{RuleDocument, RuleNumber};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuleTriggerDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advanced_filter: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_contains: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<Option<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity_type: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_list_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swimlane_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_title: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checklist_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checklist_item_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_id: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_kind: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_type: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_time: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekday: Option<Option<RuleNumber>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_of_month: Option<Option<RuleNumber>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_date: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_condition: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<Option<RuleNumber>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_key: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button_type: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button_label: Option<Option<String>>,
}
impl From<crate::client::rules::RuleTrigger> for RuleTriggerDetail {
    fn from(d: crate::client::rules::RuleTrigger) -> Self {
        Self {
            advanced_filter: d.advanced_filter,
            text_contains: d.text_contains,
            cron: d.cron,
            direction: d.direction,

            activity_type: d.activity_type,
            description: d.description,
            user_id: d.user_id,
            username: d.username,
            list_name: d.list_name,
            old_list_name: d.old_list_name,
            swimlane_name: d.swimlane_name,
            card_title: d.card_title,
            checklist_name: d.checklist_name,
            checklist_item_name: d.checklist_item_name,
            label_id: d.label_id,
            attachment_name: d.attachment_name,
            desc: d.desc,
            schedule_kind: d.schedule_kind,
            schedule_type: d.schedule_type,
            at_time: d.at_time,
            weekday: d.weekday,
            day_of_month: d.day_of_month,
            on_date: d.on_date,
            due_condition: d.due_condition,
            days: d.days,
            last_run_key: d.last_run_key,
            last_run_at: d.last_run_at,
            button_type: d.button_type,
            button_label: d.button_label,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuleActionDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_card_details: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_checklists_and_comments: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_attachments: Option<Option<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_id: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment_text: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_id: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swimlane_id: Option<Option<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swimlane_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_list_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checklist_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_item_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checklist_items: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_field: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_to: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_subject: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_msg: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_color: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_id: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_field: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<Option<RuleNumber>>,
}
impl From<crate::client::rules::RuleAction> for RuleActionDetail {
    fn from(d: crate::client::rules::RuleAction) -> Self {
        Self {
            include_card_details: d.include_card_details,
            include_checklists_and_comments: d.include_checklists_and_comments,
            include_attachments: d.include_attachments,
            integration_id: d.integration_id,
            comment_text: d.comment_text,
            list_id: d.list_id,
            swimlane_id: d.swimlane_id,

            action_type: d.action_type,
            list_name: d.list_name,
            swimlane_name: d.swimlane_name,
            card_name: d.card_name,
            from_list_name: d.from_list_name,
            checklist_name: d.checklist_name,
            check_item_name: d.check_item_name,
            checklist_items: d.checklist_items,
            date_field: d.date_field,
            unit: d.unit,
            email_to: d.email_to,
            email_subject: d.email_subject,
            email_msg: d.email_msg,
            selected_color: d.selected_color,
            label_id: d.label_id,
            username: d.username,
            sort_field: d.sort_field,
            desc: d.desc,
            days: d.days,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuleDetail {
    pub enabled: bool,
    pub extra_triggers: Option<Vec<RuleTriggerDetail>>,
    pub extra_actions: Option<Vec<RuleActionDetail>>,
    pub board_id: String,
    pub rule_id: String,
    pub title: String,
    pub trigger: Option<RuleTriggerDetail>,
    pub action: Option<RuleActionDetail>,
}
impl RuleDetail {
    pub(crate) fn from_document(board_id: String, d: RuleDocument) -> Self {
        Self {
            enabled: d.enabled,
            extra_triggers: d
                .extra_triggers
                .map(|items| items.into_iter().map(Into::into).collect()),
            extra_actions: d
                .extra_actions
                .map(|items| items.into_iter().map(Into::into).collect()),
            board_id,
            rule_id: d.rule_id,
            title: d.title,
            trigger: d.trigger.map(Into::into),
            action: d.action.map(Into::into),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuleCollectionSuccess {
    pub board_id: String,
    pub rules: Vec<RuleDetail>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuleCreateSuccess {
    pub board_id: String,
    pub rule_id: String,
    pub trigger_id: String,
    pub action_id: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSubmittedField {
    Enabled,
    Title,
    Trigger,
    Action,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuleUpdateSuccess {
    pub board_id: String,
    pub rule_id: String,
    pub submitted_fields: Vec<RuleSubmittedField>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuleDeleteSuccess {
    pub board_id: String,
    pub rule_id: String,
    pub deleted: bool,
}
