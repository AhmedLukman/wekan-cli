use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Number;

use super::{ClientError, WekanClient};
use crate::client::transport::decode_success;

/// The complete v11.06 card-settings projection. Omission is distinct from null.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardCardSettings {
    #[serde(default, deserialize_with = "optional")]
    pub allows_card_counter_list: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub card_aging: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_board_member_list: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_show_lists: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_attachments: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_checklists: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_comments: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_description_title: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_description_text: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_activities: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_labels: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_creator: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_assignee: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_members: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_requested_by: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_card_sorting_by_number: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_card_number: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_assigned_by: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_received_date: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_start_date: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_end_date: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_due_date: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_subtasks: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_creator_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_attachments_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_checklists_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_checklist_at_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_cover_attachment_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_badge_attachment_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_card_sorting_by_number_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_card_number_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_description_title_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_description_text_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_labels_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_assignee_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_members_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_requested_by_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_assigned_by_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_received_date_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_start_date_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_end_date_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_due_date_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_subtasks_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub allows_show_lists_on_minicard: Option<bool>,
    #[serde(default, deserialize_with = "optional")]
    pub card_aging_days1: Option<Number>,
    #[serde(default, deserialize_with = "optional")]
    pub card_aging_days2: Option<Number>,
    #[serde(default, deserialize_with = "optional")]
    pub card_aging_days3: Option<Number>,
}

fn optional<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

/// Partial settings update; omitted fields are never serialized.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBoardCardSettingsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_card_counter_list: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_aging: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_board_member_list: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_show_lists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_attachments: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_checklists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_comments: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_description_title: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_description_text: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_activities: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_labels: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_creator: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_assignee: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_members: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_requested_by: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_card_sorting_by_number: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_card_number: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_assigned_by: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_received_date: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_start_date: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_end_date: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_due_date: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_subtasks: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_creator_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_attachments_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_checklists_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_checklist_at_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_cover_attachment_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_badge_attachment_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_card_sorting_by_number_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_card_number_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_description_title_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_description_text_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_labels_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_assignee_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_members_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_requested_by_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_assigned_by_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_received_date_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_start_date_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_end_date_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_due_date_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_subtasks_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allows_show_lists_on_minicard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_aging_days1: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_aging_days2: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_aging_days3: Option<u64>,
}

impl WekanClient {
    pub async fn board_card_settings(
        &self,
        board: &str,
        token: &SecretString,
    ) -> Result<BoardCardSettings, ClientError> {
        super::members::validate_id(board)?;
        let body = self
            .execute_success_request(
                self.http
                    .get(self.board_endpoint(board, Some("cardSettings"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "board card settings get", true)
    }

    pub async fn update_board_card_settings(
        &self,
        board: &str,
        request: &UpdateBoardCardSettingsRequest,
        token: &SecretString,
    ) -> Result<BoardCardSettings, ClientError> {
        super::members::validate_id(board)?;
        let body = self
            .execute_success_request(
                self.http
                    .put(self.board_endpoint(board, Some("cardSettings"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        decode_success(&body, "board card settings update", true)
    }
}
