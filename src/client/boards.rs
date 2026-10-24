use crate::client::metadata::*;
use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{ClientError, WekanClient, transport::decode_success};

pub mod attachments;
pub mod card_settings;
mod copy;
pub mod domains;
mod export;
pub mod import;
pub mod presentation;
pub mod trello_import;
pub use copy::CopyBoardRequest;
pub mod labels;
pub mod members;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BoardPermission {
    #[default]
    Private,
    Public,
    Instance,
}

impl BoardPermission {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Public => "public",
            Self::Instance => "instance",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BoardColor {
    #[default]
    Belize,
    Nephritis,
    Pomegranate,
    Pumpkin,
    Wisteria,
    Moderatepink,
    Strongcyan,
    Limegreen,
    Midnight,
    Dark,
    Relax,
    Corteza,
    Appleglasspastel,
    Clearblue,
    Cleargreen,
    Clearorange,
    Clearpink,
    Clearpurple,
    Clearred,
    Natural,
    Modern,
    Moderndark,
    Exodark,
    Cleandark,
    Cleanlight,
}

impl BoardColor {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Belize => "belize",
            Self::Nephritis => "nephritis",
            Self::Pomegranate => "pomegranate",
            Self::Pumpkin => "pumpkin",
            Self::Wisteria => "wisteria",
            Self::Moderatepink => "moderatepink",
            Self::Strongcyan => "strongcyan",
            Self::Limegreen => "limegreen",
            Self::Midnight => "midnight",
            Self::Dark => "dark",
            Self::Relax => "relax",
            Self::Corteza => "corteza",
            Self::Appleglasspastel => "appleglasspastel",
            Self::Clearblue => "clearblue",
            Self::Cleargreen => "cleargreen",
            Self::Clearorange => "clearorange",
            Self::Clearpink => "clearpink",
            Self::Clearpurple => "clearpurple",
            Self::Clearred => "clearred",
            Self::Natural => "natural",
            Self::Modern => "modern",
            Self::Moderndark => "moderndark",
            Self::Exodark => "exodark",
            Self::Cleandark => "cleandark",
            Self::Cleanlight => "cleanlight",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PresentParentTask {
    PrefixWithFullPath,
    PrefixWithParent,
    SubtextWithFullPath,
    SubtextWithParent,
    NoParent,
}

impl PresentParentTask {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::PrefixWithFullPath => "prefix-with-full-path",
            Self::PrefixWithParent => "prefix-with-parent",
            Self::SubtextWithFullPath => "subtext-with-full-path",
            Self::SubtextWithParent => "subtext-with-parent",
            Self::NoParent => "no-parent",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoardType {
    Board,
    TemplateBoard,
    TemplateContainer,
}

impl BoardType {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Board => "board",
            Self::TemplateBoard => "template-board",
            Self::TemplateContainer => "template-container",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BoardWatchLevel {
    Watching,
    Tracking,
    Muted,
}

impl BoardWatchLevel {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Watching => "watching",
            Self::Tracking => "tracking",
            Self::Muted => "muted",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBoardRequest {
    pub title: String,
    pub permission: BoardPermission,
    pub color: BoardColor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateBoardResult {
    pub board_id: String,
    pub default_swimlane_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenameBoardResult {
    pub board_id: String,
    pub title: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeleteBoardResult {
    pub board_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoardCounts {
    pub instance: u64,
    pub private: u64,
    pub public: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BoardSummary {
    #[serde(rename = "_id")]
    pub board_id: String,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardMember {
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub granted_by: Option<Vec<String>>,
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub custom_role_id: Option<String>,
    pub user_id: String,
    pub is_admin: bool,
    pub is_active: bool,
    pub is_no_comments: Option<bool>,
    pub is_comment_only: Option<bool>,
    pub is_worker: Option<bool>,
    pub is_normal_assigned_only: Option<bool>,
    pub is_comment_assigned_only: Option<bool>,
    pub is_read_only: Option<bool>,
    pub is_read_assigned_only: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardWatcher {
    pub user_id: String,
    pub level: BoardWatchLevel,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardLabel {
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub due_at: Option<String>,
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub shared_label_id: Option<String>,
    #[serde(rename(deserialize = "_id", serialize = "label_id"))]
    pub label_id: String,
    pub name: Option<String>,
    pub color: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardOrganization {
    pub org_id: String,
    pub org_display_name: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardTeam {
    pub team_id: String,
    pub team_display_name: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardDomain {
    pub domain: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardDocument {
    pub scrum: Option<Box<ScrumSettings>>,
    pub scrum_import_losses: Option<Vec<ScrumImportLoss>>,
    pub announcement: Option<BoardAnnouncement>,
    pub wip_limit_groups: Option<Vec<BoardWipGroup>>,
    pub email_to_card: Option<Box<BoardEmailToCard>>,
    pub notification_delivery: Option<Box<NotificationDelivery>>,
    pub column_sets: Option<Vec<BoardColumnSet>>,

    pub scrum_revision: Option<serde_json::Number>,
    pub allows_instance_editing: Option<bool>,
    pub import_run_id: Option<String>,
    pub sync_effects_enabled: Option<bool>,
    pub map_image_attachment_id: Option<String>,
    pub auto_archive_inactive_days: Option<Number>,
    pub rules_visible_to_members: Option<bool>,
    pub card_field_order: Option<Vec<String>>,
    pub minicard_field_order: Option<Vec<String>>,
    pub notify_override_tray: Option<bool>,
    pub notify_override_email: Option<bool>,
    pub due_reminder_days: Option<Vec<Number>>,
    pub due_reminder_webhook: Option<bool>,
    pub labels_above_title_on_card: Option<bool>,
    pub allows_checklist_due_date_on_minicard: Option<bool>,
    pub allows_location_on_minicard: Option<bool>,
    pub allows_swimlane_name_on_card: Option<bool>,
    pub allows_flowtime_on_minicard: Option<bool>,
    pub allows_pomodoro_on_minicard: Option<bool>,
    pub allows_attachment_list_on_minicard: Option<bool>,
    pub allows_text_notes_on_minicard: Option<bool>,
    pub allows_comment_count_on_card: Option<bool>,
    pub allows_activities_on_minicard: Option<bool>,
    pub allows_card_buttons_on_minicard: Option<bool>,
    pub allows_description_badge_on_card: Option<bool>,
    pub allows_unread_comments_on_card: Option<bool>,
    pub allows_due_from_title_on_card: Option<bool>,
    pub allows_due_from_title_on_minicard: Option<bool>,
    pub allows_similar_cards_on_card: Option<bool>,
    pub allows_similar_cards_on_minicard: Option<bool>,
    pub allows_at_risk_on_card: Option<bool>,
    pub allows_at_risk_on_minicard: Option<bool>,
    pub allows_suggested_members_on_card: Option<bool>,
    pub allows_forecast_on_card: Option<bool>,
    pub allows_forecast_on_minicard: Option<bool>,
    pub allows_checklist_from_description_on_card: Option<bool>,
    pub allows_action_items_on_card: Option<bool>,
    pub allows_whiteboard_on_card: Option<bool>,
    pub shared_list_headers: Option<bool>,
    pub board_view_settings: Option<
        std::collections::BTreeMap<
            String,
            crate::client::boards::presentation::StoredBoardViewVisibility,
        >,
    >,
    pub board_view_order: Option<Vec<String>>,
    pub date_format_override: Option<bool>,
    pub date_format: Option<String>,
    pub ask_for_move_reason: Option<bool>,
    pub subtasks_default_swimlane_id: Option<String>,
    pub allows_swimlane_dragging: Option<bool>,
    pub allows_list_dragging: Option<bool>,
    pub allows_card_dragging: Option<bool>,
    pub allows_checklist_dragging: Option<bool>,
    pub allows_checklist_item_dragging: Option<bool>,
    pub allows_subtask_dragging: Option<bool>,
    pub allows_minicard_collapse: Option<bool>,
    pub labels_above_title_on_minicard: Option<bool>,
    pub allows_checklist_due_date: Option<bool>,
    pub allows_checklist_title: Option<bool>,
    pub allows_comments_on_minicard: Option<bool>,
    pub show_label_text: Option<bool>,
    pub allows_swimlane_name_on_minicard: Option<bool>,
    pub allows_spent_time: Option<bool>,
    pub allows_spent_time_on_minicard: Option<bool>,
    pub allows_stickers: Option<bool>,
    pub allows_stickers_on_minicard: Option<bool>,
    pub allows_location: Option<bool>,
    pub allows_dependencies: Option<bool>,
    pub allows_dependencies_on_minicard: Option<bool>,
    pub allows_flowtime: Option<bool>,
    pub allows_pomodoro: Option<bool>,
    pub allows_vote: Option<bool>,
    pub allows_vote_on_minicard: Option<bool>,
    pub allows_poker: Option<bool>,
    pub allows_poker_on_minicard: Option<bool>,
    pub allows_text_notes: Option<bool>,
    pub allows_comment_count_on_minicard: Option<bool>,
    pub allows_card_collapse: Option<bool>,
    pub allows_checklist_title_on_minicard: Option<bool>,
    pub allows_label_text_on_card: Option<bool>,
    pub allows_card_color_on_card: Option<bool>,
    pub allows_card_color_on_minicard: Option<bool>,
    pub allows_parent_card_on_card: Option<bool>,
    pub allows_parent_card_on_minicard: Option<bool>,
    pub allows_linked_card_on_card: Option<bool>,
    pub allows_linked_card_on_minicard: Option<bool>,
    pub allows_card_buttons_on_card: Option<bool>,
    pub allows_description_badge_on_minicard: Option<bool>,
    pub allows_unread_comments_on_minicard: Option<bool>,
    pub allows_issue_type_on_minicard: Option<bool>,
    pub sticky_list_headers: Option<bool>,
    pub list_width_resize_locked: Option<bool>,
    pub swimlane_height_resize_locked: Option<bool>,
    pub default_public_board_view: Option<String>,
    pub default_private_board_view: Option<String>,
    pub same_width_for_all_lists: Option<bool>,
    pub same_width_for_all_lists_value: Option<Number>,
    pub key: Option<String>,
    #[serde(rename(deserialize = "_id", serialize = "board_id"))]
    pub board_id: String,
    pub title: String,
    pub slug: Option<String>,
    pub archived: Option<bool>,
    pub archived_at: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub stars: Option<Number>,
    #[serde(default)]
    pub labels: Vec<BoardLabel>,
    pub members: Vec<BoardMember>,
    #[serde(default)]
    pub watchers: Vec<BoardWatcher>,
    pub permission: Option<BoardPermission>,
    #[serde(default)]
    pub orgs: Vec<BoardOrganization>,
    #[serde(default)]
    pub teams: Vec<BoardTeam>,
    #[serde(default)]
    pub domains: Vec<BoardDomain>,
    #[serde(default)]
    pub import_usernames: Vec<String>,
    pub color: Option<BoardColor>,
    #[serde(default)]
    pub custom_theme_colors: Vec<String>,
    #[serde(rename(deserialize = "backgroundImageURL", serialize = "background_image_url"))]
    pub background_image_url: Option<String>,
    pub background_image_id: Option<String>,
    pub allows_card_counter_list: Option<bool>,
    pub card_aging: Option<bool>,
    pub show_dependencies: Option<bool>,
    pub card_aging_days1: Option<Number>,
    pub card_aging_days2: Option<Number>,
    pub card_aging_days3: Option<Number>,
    pub allows_board_member_list: Option<bool>,
    pub description: Option<String>,
    pub subtasks_default_board_id: Option<String>,
    pub migration_version: Option<Number>,
    pub subtasks_default_list_id: Option<String>,
    pub date_settings_default_board_id: Option<String>,
    pub date_settings_default_list_id: Option<String>,
    pub allows_subtasks: Option<bool>,
    pub allows_subtasks_on_minicard: Option<bool>,
    pub allows_attachments: Option<bool>,
    pub allows_attachments_on_minicard: Option<bool>,
    pub allows_checklists: Option<bool>,
    pub allows_checklists_on_minicard: Option<bool>,
    pub allows_custom_fields: Option<bool>,
    pub allows_custom_fields_on_minicard: Option<bool>,
    pub allows_checklist_count_badge_on_minicard: Option<bool>,
    pub allows_comments: Option<bool>,
    pub allows_description_title: Option<bool>,
    pub allows_description_title_on_minicard: Option<bool>,
    pub allows_description_text: Option<bool>,
    pub allows_description_text_on_minicard: Option<bool>,
    pub allows_cover_attachment_on_minicard: Option<bool>,
    pub allows_cover_attachment_on_card: Option<bool>,
    pub allows_badge_attachment_on_minicard: Option<bool>,
    pub allows_attachment_count_on_card: Option<bool>,
    pub allows_checklist_count_badge_on_card: Option<bool>,
    pub allows_card_sorting_by_number_on_minicard: Option<bool>,
    pub allows_card_number: Option<bool>,
    pub allows_card_number_on_minicard: Option<bool>,
    pub allows_activities: Option<bool>,
    pub allows_labels: Option<bool>,
    pub allows_labels_on_minicard: Option<bool>,
    pub allows_creator: Option<bool>,
    pub allows_creator_on_minicard: Option<bool>,
    pub allows_assignee: Option<bool>,
    pub allows_assignee_on_minicard: Option<bool>,
    pub allows_members: Option<bool>,
    pub allows_members_on_minicard: Option<bool>,
    pub allows_requested_by: Option<bool>,
    pub allows_requested_by_on_minicard: Option<bool>,
    pub allows_card_sorting_by_number: Option<bool>,
    pub allows_show_lists: Option<bool>,
    pub allows_assigned_by: Option<bool>,
    pub allows_assigned_by_on_minicard: Option<bool>,
    pub allows_show_lists_on_minicard: Option<bool>,
    pub allows_checklist_at_minicard: Option<bool>,
    pub allows_received_date: Option<bool>,
    pub restrict_comment_editing: Option<bool>,
    pub allows_personal_list_width: Option<bool>,
    pub auto_width: Option<bool>,
    pub allows_received_date_on_minicard: Option<bool>,
    pub allows_start_date: Option<bool>,
    pub allows_start_date_on_minicard: Option<bool>,
    pub allows_end_date: Option<bool>,
    pub allows_end_date_on_minicard: Option<bool>,
    pub allows_due_date: Option<bool>,
    pub allows_due_date_on_minicard: Option<bool>,
    pub allows_due_complete: Option<bool>,
    pub allows_due_complete_on_minicard: Option<bool>,
    pub present_parent_task: Option<PresentParentTask>,
    pub received_at: Option<String>,
    pub start_at: Option<String>,
    pub due_at: Option<String>,
    pub end_at: Option<String>,
    pub spent_time: Option<Number>,
    pub is_overtime: Option<bool>,
    #[serde(rename(deserialize = "type", serialize = "board_type"))]
    pub board_type: Option<BoardType>,
    pub sort: Option<Number>,
    pub show_activities: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct CreateBoardResponse {
    #[serde(rename = "_id")]
    board_id: String,
    default_swimlane_id: String,
}

#[derive(Serialize)]
struct RenameBoardRequest<'a> {
    title: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameBoardResponse {
    #[serde(rename = "_id")]
    board_id: String,
    title: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdResponse {
    #[serde(rename = "_id")]
    id: String,
}

impl WekanClient {
    pub async fn public_boards(
        &self,
        token: &SecretString,
    ) -> Result<Vec<BoardSummary>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.boards_api_endpoint("api/boards", "public boards")?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let boards = decode_success(&body, "public boards", true)?;
        validate_board_summaries(boards, "public boards")
    }

    pub async fn board_counts(&self, token: &SecretString) -> Result<BoardCounts, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.boards_api_endpoint("api/boards_count", "board counts")?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "board counts", true)
    }

    pub async fn board(
        &self,
        board_id: &str,
        token: &SecretString,
    ) -> Result<BoardDocument, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.board_endpoint(board_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let board = decode_success(&body, "board", true)?;
        validate_board_document(board)
    }

    pub async fn create_board(
        &self,
        request: &CreateBoardRequest,
        token: &SecretString,
    ) -> Result<CreateBoardResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.boards_api_endpoint("api/boards", "board creation")?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: CreateBoardResponse = decode_success(&body, "board creation", true)?;
        require_non_empty(&response.board_id, "board creation", "board id")?;
        require_non_empty(
            &response.default_swimlane_id,
            "board creation",
            "default swimlane id",
        )?;
        Ok(CreateBoardResult {
            board_id: response.board_id,
            default_swimlane_id: response.default_swimlane_id,
        })
    }

    pub async fn rename_board(
        &self,
        board_id: &str,
        title: &str,
        token: &SecretString,
    ) -> Result<RenameBoardResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.board_endpoint(board_id, Some("title"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(&RenameBoardRequest { title }),
            )
            .await?;
        let response: RenameBoardResponse = decode_success(&body, "board rename", true)?;
        require_non_empty(&response.board_id, "board rename", "board id")?;
        require_non_empty(&response.title, "board rename", "title")?;
        Ok(RenameBoardResult {
            board_id: response.board_id,
            title: response.title,
        })
    }

    pub async fn delete_board(
        &self,
        board_id: &str,
        token: &SecretString,
    ) -> Result<DeleteBoardResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .delete(self.board_endpoint(board_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "board deletion", true)?;
        require_non_empty(&response.id, "board deletion", "board id")?;
        Ok(DeleteBoardResult {
            board_id: response.id,
        })
    }

    fn boards_api_endpoint(
        &self,
        path: &str,
        operation: &str,
    ) -> Result<reqwest::Url, ClientError> {
        self.server()
            .join(path)
            .map_err(|error| ClientError::Protocol {
                diagnostic: None,
                message: format!("could not build the {operation} endpoint: {error}"),
                success_status_received: false,
            })
    }

    pub(super) fn board_endpoint(
        &self,
        board_id: &str,
        suffix: Option<&str>,
    ) -> Result<reqwest::Url, ClientError> {
        let mut endpoint = self.boards_api_endpoint("api/boards", "board")?;
        let mut segments = endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                diagnostic: None,
                message: "could not add the board id to the endpoint".to_owned(),
                success_status_received: false,
            })?;
        segments.push(board_id);
        if let Some(suffix) = suffix {
            segments.push(suffix);
        }
        drop(segments);
        Ok(endpoint)
    }
}

fn validate_board_summaries(
    boards: Vec<BoardSummary>,
    operation: &str,
) -> Result<Vec<BoardSummary>, ClientError> {
    for board in &boards {
        require_non_empty(&board.board_id, operation, "board id")?;
        require_non_empty(&board.title, operation, "title")?;
    }
    Ok(boards)
}

fn validate_board_document(board: BoardDocument) -> Result<BoardDocument, ClientError> {
    require_non_empty(&board.board_id, "board", "board id")?;
    require_non_empty(&board.title, "board", "title")?;
    for label in &board.labels {
        if let Some(value) = &label.due_at {
            validate_date_time(value, "label dueAt")?;
        }
    }
    for (field, value) in [
        ("archivedAt", board.archived_at.as_deref()),
        ("createdAt", board.created_at.as_deref()),
        ("modifiedAt", board.modified_at.as_deref()),
        ("receivedAt", board.received_at.as_deref()),
        ("startAt", board.start_at.as_deref()),
        ("dueAt", board.due_at.as_deref()),
        ("endAt", board.end_at.as_deref()),
    ] {
        if let Some(value) = value {
            validate_date_time(value, field)?;
        }
    }
    Ok(board)
}

fn validate_date_time(value: &str, field: &str) -> Result<(), ClientError> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map(|_| ())
        .map_err(|_| ClientError::Protocol {
            diagnostic: None,
            message: format!("the board response contained an invalid {field} date-time"),
            success_status_received: true,
        })
}

fn require_non_empty(value: &str, operation: &str, field: &str) -> Result<(), ClientError> {
    if value.is_empty() {
        Err(ClientError::Protocol {
            diagnostic: None,
            message: format!("the {operation} response contained an empty {field}"),
            success_status_received: true,
        })
    } else {
        Ok(())
    }
}
