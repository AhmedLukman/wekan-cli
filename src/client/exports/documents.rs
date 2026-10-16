//! Explicit projected response objects for Wekan v11.06 exports.
//! Derived from the corrected contract; schemaless built-in rule/activity fields
//! are verified against the pinned producers, not captured in a flattened map.
#![allow(dead_code)] // Validation-only DTO values are dropped after each record.
use super::validation::{Base64, Date, DiscardVec, Never, optional, required_nullable};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BoardExportDocument {
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "title")]
    _title: String,
    #[serde(rename = "slug", default, deserialize_with = "optional")]
    _slug: Option<String>,
    #[serde(rename = "archived", default, deserialize_with = "optional")]
    _archived: Option<bool>,
    #[serde(rename = "archivedAt", default, deserialize_with = "optional")]
    _archived_at: Option<Date>,
    #[serde(rename = "createdAt", default, deserialize_with = "optional")]
    _created_at: Option<Date>,
    #[serde(rename = "modifiedAt", default, deserialize_with = "optional")]
    _modified_at: Option<Date>,
    #[serde(rename = "labels", default, deserialize_with = "optional")]
    _labels: Option<DiscardVec<BoardLabel>>,
    #[serde(rename = "members")]
    _members: DiscardVec<BoardMember>,
    #[serde(rename = "watchers", default, deserialize_with = "optional")]
    _watchers: Option<DiscardVec<BoardWatcher>>,
    #[serde(rename = "permission", default, deserialize_with = "optional")]
    _permission: Option<BoardExportDocumentPermission>,
    #[serde(rename = "orgs", default, deserialize_with = "optional")]
    _orgs: Option<DiscardVec<BoardOrganization>>,
    #[serde(rename = "teams", default, deserialize_with = "optional")]
    _teams: Option<DiscardVec<BoardTeam>>,
    #[serde(rename = "domains", default, deserialize_with = "optional")]
    _domains: Option<DiscardVec<BoardDomain>>,
    #[serde(rename = "importUsernames", default, deserialize_with = "optional")]
    _import_usernames: Option<DiscardVec<String>>,
    #[serde(rename = "color", default, deserialize_with = "optional")]
    _color: Option<BoardExportDocumentColor>,
    #[serde(rename = "customThemeColors", default, deserialize_with = "optional")]
    _custom_theme_colors: Option<DiscardVec<String>>,
    #[serde(rename = "backgroundImageURL", default, deserialize_with = "optional")]
    _background_image_url: Option<String>,
    #[serde(rename = "backgroundImageId", default, deserialize_with = "optional")]
    _background_image_id: Option<String>,
    #[serde(
        rename = "allowsCardCounterList",
        default,
        deserialize_with = "optional"
    )]
    _allows_card_counter_list: Option<bool>,
    #[serde(rename = "cardAging", default, deserialize_with = "optional")]
    _card_aging: Option<bool>,
    #[serde(rename = "showDependencies", default, deserialize_with = "optional")]
    _show_dependencies: Option<bool>,
    #[serde(rename = "cardAgingDays1", default, deserialize_with = "optional")]
    _card_aging_days1: Option<serde_json::Number>,
    #[serde(rename = "cardAgingDays2", default, deserialize_with = "optional")]
    _card_aging_days2: Option<serde_json::Number>,
    #[serde(rename = "cardAgingDays3", default, deserialize_with = "optional")]
    _card_aging_days3: Option<serde_json::Number>,
    #[serde(
        rename = "allowsBoardMemberList",
        default,
        deserialize_with = "optional"
    )]
    _allows_board_member_list: Option<bool>,
    #[serde(rename = "description", default, deserialize_with = "optional")]
    _description: Option<String>,
    #[serde(
        rename = "subtasksDefaultBoardId",
        default,
        deserialize_with = "optional"
    )]
    _subtasks_default_board_id: Option<Option<String>>,
    #[serde(rename = "migrationVersion", default, deserialize_with = "optional")]
    _migration_version: Option<serde_json::Number>,
    #[serde(
        rename = "subtasksDefaultListId",
        default,
        deserialize_with = "optional"
    )]
    _subtasks_default_list_id: Option<Option<String>>,
    #[serde(
        rename = "dateSettingsDefaultBoardId",
        default,
        deserialize_with = "optional"
    )]
    _date_settings_default_board_id: Option<Option<String>>,
    #[serde(
        rename = "dateSettingsDefaultListId",
        default,
        deserialize_with = "optional"
    )]
    _date_settings_default_list_id: Option<Option<String>>,
    #[serde(rename = "allowsSubtasks", default, deserialize_with = "optional")]
    _allows_subtasks: Option<bool>,
    #[serde(
        rename = "allowsSubtasksOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_subtasks_on_minicard: Option<bool>,
    #[serde(rename = "allowsAttachments", default, deserialize_with = "optional")]
    _allows_attachments: Option<bool>,
    #[serde(
        rename = "allowsAttachmentsOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_attachments_on_minicard: Option<bool>,
    #[serde(rename = "allowsChecklists", default, deserialize_with = "optional")]
    _allows_checklists: Option<bool>,
    #[serde(
        rename = "allowsChecklistsOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_checklists_on_minicard: Option<bool>,
    #[serde(rename = "allowsCustomFields", default, deserialize_with = "optional")]
    _allows_custom_fields: Option<bool>,
    #[serde(
        rename = "allowsCustomFieldsOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_custom_fields_on_minicard: Option<bool>,
    #[serde(
        rename = "allowsChecklistCountBadgeOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_checklist_count_badge_on_minicard: Option<bool>,
    #[serde(rename = "allowsComments", default, deserialize_with = "optional")]
    _allows_comments: Option<bool>,
    #[serde(
        rename = "allowsDescriptionTitle",
        default,
        deserialize_with = "optional"
    )]
    _allows_description_title: Option<bool>,
    #[serde(
        rename = "allowsDescriptionTitleOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_description_title_on_minicard: Option<bool>,
    #[serde(
        rename = "allowsDescriptionText",
        default,
        deserialize_with = "optional"
    )]
    _allows_description_text: Option<bool>,
    #[serde(
        rename = "allowsDescriptionTextOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_description_text_on_minicard: Option<bool>,
    #[serde(
        rename = "allowsCoverAttachmentOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_cover_attachment_on_minicard: Option<bool>,
    #[serde(
        rename = "allowsCoverAttachmentOnCard",
        default,
        deserialize_with = "optional"
    )]
    _allows_cover_attachment_on_card: Option<bool>,
    #[serde(
        rename = "allowsBadgeAttachmentOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_badge_attachment_on_minicard: Option<bool>,
    #[serde(
        rename = "allowsAttachmentCountOnCard",
        default,
        deserialize_with = "optional"
    )]
    _allows_attachment_count_on_card: Option<bool>,
    #[serde(
        rename = "allowsChecklistCountBadgeOnCard",
        default,
        deserialize_with = "optional"
    )]
    _allows_checklist_count_badge_on_card: Option<bool>,
    #[serde(
        rename = "allowsCardSortingByNumberOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_card_sorting_by_number_on_minicard: Option<bool>,
    #[serde(rename = "allowsCardNumber", default, deserialize_with = "optional")]
    _allows_card_number: Option<bool>,
    #[serde(
        rename = "allowsCardNumberOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_card_number_on_minicard: Option<bool>,
    #[serde(rename = "allowsActivities", default, deserialize_with = "optional")]
    _allows_activities: Option<bool>,
    #[serde(rename = "allowsLabels", default, deserialize_with = "optional")]
    _allows_labels: Option<bool>,
    #[serde(
        rename = "allowsLabelsOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_labels_on_minicard: Option<bool>,
    #[serde(rename = "allowsCreator", default, deserialize_with = "optional")]
    _allows_creator: Option<bool>,
    #[serde(
        rename = "allowsCreatorOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_creator_on_minicard: Option<bool>,
    #[serde(rename = "allowsAssignee", default, deserialize_with = "optional")]
    _allows_assignee: Option<bool>,
    #[serde(
        rename = "allowsAssigneeOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_assignee_on_minicard: Option<bool>,
    #[serde(rename = "allowsMembers", default, deserialize_with = "optional")]
    _allows_members: Option<bool>,
    #[serde(
        rename = "allowsMembersOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_members_on_minicard: Option<bool>,
    #[serde(rename = "allowsRequestedBy", default, deserialize_with = "optional")]
    _allows_requested_by: Option<bool>,
    #[serde(
        rename = "allowsRequestedByOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_requested_by_on_minicard: Option<bool>,
    #[serde(
        rename = "allowsCardSortingByNumber",
        default,
        deserialize_with = "optional"
    )]
    _allows_card_sorting_by_number: Option<bool>,
    #[serde(rename = "allowsShowLists", default, deserialize_with = "optional")]
    _allows_show_lists: Option<bool>,
    #[serde(rename = "allowsAssignedBy", default, deserialize_with = "optional")]
    _allows_assigned_by: Option<bool>,
    #[serde(
        rename = "allowsAssignedByOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_assigned_by_on_minicard: Option<bool>,
    #[serde(
        rename = "allowsShowListsOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_show_lists_on_minicard: Option<bool>,
    #[serde(
        rename = "allowsChecklistAtMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_checklist_at_minicard: Option<bool>,
    #[serde(rename = "allowsReceivedDate", default, deserialize_with = "optional")]
    _allows_received_date: Option<bool>,
    #[serde(
        rename = "restrictCommentEditing",
        default,
        deserialize_with = "optional"
    )]
    _restrict_comment_editing: Option<bool>,
    #[serde(
        rename = "allowsPersonalListWidth",
        default,
        deserialize_with = "optional"
    )]
    _allows_personal_list_width: Option<bool>,
    #[serde(rename = "autoWidth", default, deserialize_with = "optional")]
    _auto_width: Option<bool>,
    #[serde(
        rename = "allowsReceivedDateOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_received_date_on_minicard: Option<bool>,
    #[serde(rename = "allowsStartDate", default, deserialize_with = "optional")]
    _allows_start_date: Option<bool>,
    #[serde(
        rename = "allowsStartDateOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_start_date_on_minicard: Option<bool>,
    #[serde(rename = "allowsEndDate", default, deserialize_with = "optional")]
    _allows_end_date: Option<bool>,
    #[serde(
        rename = "allowsEndDateOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_end_date_on_minicard: Option<bool>,
    #[serde(rename = "allowsDueDate", default, deserialize_with = "optional")]
    _allows_due_date: Option<bool>,
    #[serde(
        rename = "allowsDueDateOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_due_date_on_minicard: Option<bool>,
    #[serde(rename = "allowsDueComplete", default, deserialize_with = "optional")]
    _allows_due_complete: Option<bool>,
    #[serde(
        rename = "allowsDueCompleteOnMinicard",
        default,
        deserialize_with = "optional"
    )]
    _allows_due_complete_on_minicard: Option<bool>,
    #[serde(rename = "presentParentTask", default, deserialize_with = "optional")]
    _present_parent_task: Option<BoardExportDocumentPresentParentTask>,
    #[serde(rename = "receivedAt", default, deserialize_with = "optional")]
    _received_at: Option<Date>,
    #[serde(rename = "startAt", default, deserialize_with = "optional")]
    _start_at: Option<Date>,
    #[serde(rename = "dueAt", default, deserialize_with = "optional")]
    _due_at: Option<Date>,
    #[serde(rename = "endAt", default, deserialize_with = "optional")]
    _end_at: Option<Date>,
    #[serde(rename = "spentTime", default, deserialize_with = "optional")]
    _spent_time: Option<serde_json::Number>,
    #[serde(rename = "isOvertime", default, deserialize_with = "optional")]
    _is_overtime: Option<bool>,
    #[serde(rename = "type", default, deserialize_with = "optional")]
    _type: Option<BoardExportDocumentType>,
    #[serde(rename = "sort", default, deserialize_with = "optional")]
    _sort: Option<serde_json::Number>,
    #[serde(rename = "showActivities", default, deserialize_with = "optional")]
    _show_activities: Option<bool>,
    #[serde(rename = "_format")]
    __format: BoardExportDocumentFormat,
    #[serde(rename = "attachments")]
    _attachments: DiscardVec<ExportAttachment>,
    #[serde(rename = "lists")]
    _lists: DiscardVec<ExportList>,
    #[serde(rename = "swimlanes")]
    _swimlanes: DiscardVec<ExportSwimlane>,
    #[serde(rename = "customFields")]
    _custom_fields: DiscardVec<ExportCustomField>,
    #[serde(rename = "cards")]
    _cards: DiscardVec<ExportCard>,
    #[serde(rename = "comments")]
    _comments: DiscardVec<ExportComment>,
    #[serde(rename = "activities")]
    _activities: DiscardVec<ExportActivity>,
    #[serde(rename = "checklists")]
    _checklists: DiscardVec<Checklists>,
    #[serde(rename = "checklistItems")]
    _checklist_items: DiscardVec<ChecklistItems>,
    #[serde(rename = "subtaskItems")]
    _subtask_items: DiscardVec<Cards>,
    #[serde(rename = "rules")]
    _rules: DiscardVec<ExportRule>,
    #[serde(rename = "triggers")]
    _triggers: DiscardVec<ExportTrigger>,
    #[serde(rename = "actions")]
    _actions: DiscardVec<ExportAction>,
    #[serde(rename = "users")]
    _users: DiscardVec<ExportUser>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KanboardExport {
    #[serde(rename = "board")]
    _board: KanboardExportBoard,
    #[serde(rename = "columns")]
    _columns: DiscardVec<KanboardExportColumnsItem>,
    #[serde(rename = "swimlanes")]
    _swimlanes: DiscardVec<KanboardExportSwimlanesItem>,
    #[serde(rename = "tasks")]
    _tasks: DiscardVec<KanboardExportTasksItem>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum SingleAttachmentExport {
    Choice0(SingleAttachmentExportChoice0),
    Choice1(SingleAttachmentExportChoice1),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GithubIssueExport {
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    _title: Option<String>,
    #[serde(rename = "body")]
    _body: String,
    #[serde(rename = "state")]
    _state: GithubIssueExportState,
    #[serde(rename = "labels")]
    _labels: DiscardVec<GithubIssueExportLabelsItem>,
    #[serde(rename = "due_date", default, deserialize_with = "optional")]
    _due_date: Option<Date>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GitlabIssueExport {
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    _title: Option<String>,
    #[serde(rename = "description")]
    _description: String,
    #[serde(rename = "state")]
    _state: GitlabIssueExportState,
    #[serde(rename = "labels")]
    _labels: DiscardVec<String>,
    #[serde(rename = "due_date", default, deserialize_with = "optional")]
    _due_date: Option<Date>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeckExport {
    #[serde(rename = "title")]
    _title: String,
    #[serde(rename = "stacks")]
    _stacks: DiscardVec<DeckExportStacksItem>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OpenprojectExport {
    #[serde(rename = "_embedded")]
    __embedded: OpenprojectExportEmbedded,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TrelloExport {
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "prefs")]
    _prefs: TrelloExportPrefs,
    #[serde(rename = "labels")]
    _labels: DiscardVec<TrelloExportLabelsItem>,
    #[serde(rename = "lists")]
    _lists: DiscardVec<TrelloExportListsItem>,
    #[serde(rename = "cards")]
    _cards: DiscardVec<TrelloExportCardsItem>,
    #[serde(rename = "checklists")]
    _checklists: DiscardVec<Never>,
    #[serde(rename = "actions")]
    _actions: DiscardVec<Never>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JiraExport {
    #[serde(rename = "board")]
    _board: JiraExportBoard,
    #[serde(rename = "issues")]
    _issues: DiscardVec<JiraExportIssuesItem>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AsanaExport {
    #[serde(rename = "data")]
    _data: DiscardVec<AsanaExportDataItem>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ZenkitExport {
    #[serde(rename = "title")]
    _title: String,
    #[serde(rename = "stages")]
    _stages: DiscardVec<ZenkitExportStagesItem>,
    #[serde(rename = "items")]
    _items: DiscardVec<ZenkitExportItemsItem>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BoardLabel {
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "name", default, deserialize_with = "optional")]
    _name: Option<String>,
    #[serde(rename = "color")]
    _color: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BoardMember {
    #[serde(rename = "userId")]
    _user_id: String,
    #[serde(rename = "isAdmin")]
    _is_admin: bool,
    #[serde(rename = "isActive")]
    _is_active: bool,
    #[serde(rename = "isNoComments", default, deserialize_with = "optional")]
    _is_no_comments: Option<bool>,
    #[serde(rename = "isCommentOnly", default, deserialize_with = "optional")]
    _is_comment_only: Option<bool>,
    #[serde(rename = "isWorker", default, deserialize_with = "optional")]
    _is_worker: Option<bool>,
    #[serde(
        rename = "isNormalAssignedOnly",
        default,
        deserialize_with = "optional"
    )]
    _is_normal_assigned_only: Option<bool>,
    #[serde(
        rename = "isCommentAssignedOnly",
        default,
        deserialize_with = "optional"
    )]
    _is_comment_assigned_only: Option<bool>,
    #[serde(rename = "isReadOnly", default, deserialize_with = "optional")]
    _is_read_only: Option<bool>,
    #[serde(rename = "isReadAssignedOnly", default, deserialize_with = "optional")]
    _is_read_assigned_only: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BoardWatcher {
    #[serde(rename = "userId")]
    _user_id: String,
    #[serde(rename = "level")]
    _level: BoardWatcherLevel,
}

#[derive(Deserialize)]
pub(super) enum BoardExportDocumentPermission {
    #[serde(rename = "private")]
    Choice0,
    #[serde(rename = "public")]
    Choice1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BoardOrganization {
    #[serde(rename = "orgId")]
    _org_id: String,
    #[serde(rename = "orgDisplayName")]
    _org_display_name: String,
    #[serde(rename = "isActive")]
    _is_active: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BoardTeam {
    #[serde(rename = "teamId")]
    _team_id: String,
    #[serde(rename = "teamDisplayName")]
    _team_display_name: String,
    #[serde(rename = "isActive")]
    _is_active: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BoardDomain {
    #[serde(rename = "domain")]
    _domain: String,
    #[serde(rename = "isActive")]
    _is_active: bool,
}

#[derive(Deserialize)]
pub(super) enum BoardExportDocumentColor {
    #[serde(rename = "belize")]
    Choice0,
    #[serde(rename = "nephritis")]
    Choice1,
    #[serde(rename = "pomegranate")]
    Choice2,
    #[serde(rename = "pumpkin")]
    Choice3,
    #[serde(rename = "wisteria")]
    Choice4,
    #[serde(rename = "moderatepink")]
    Choice5,
    #[serde(rename = "strongcyan")]
    Choice6,
    #[serde(rename = "limegreen")]
    Choice7,
    #[serde(rename = "midnight")]
    Choice8,
    #[serde(rename = "dark")]
    Choice9,
    #[serde(rename = "relax")]
    Choice10,
    #[serde(rename = "corteza")]
    Choice11,
    #[serde(rename = "appleglasspastel")]
    Choice12,
    #[serde(rename = "clearblue")]
    Choice13,
    #[serde(rename = "cleargreen")]
    Choice14,
    #[serde(rename = "clearorange")]
    Choice15,
    #[serde(rename = "clearpink")]
    Choice16,
    #[serde(rename = "clearpurple")]
    Choice17,
    #[serde(rename = "clearred")]
    Choice18,
    #[serde(rename = "natural")]
    Choice19,
    #[serde(rename = "modern")]
    Choice20,
    #[serde(rename = "moderndark")]
    Choice21,
    #[serde(rename = "exodark")]
    Choice22,
    #[serde(rename = "cleandark")]
    Choice23,
    #[serde(rename = "cleanlight")]
    Choice24,
}

#[derive(Deserialize)]
pub(super) enum BoardExportDocumentPresentParentTask {
    #[serde(rename = "prefix-with-full-path")]
    Choice0,
    #[serde(rename = "prefix-with-parent")]
    Choice1,
    #[serde(rename = "subtext-with-full-path")]
    Choice2,
    #[serde(rename = "subtext-with-parent")]
    Choice3,
    #[serde(rename = "no-parent")]
    Choice4,
}

#[derive(Deserialize)]
pub(super) enum BoardExportDocumentType {
    #[serde(rename = "board")]
    Choice0,
    #[serde(rename = "template-board")]
    Choice1,
    #[serde(rename = "template-container")]
    Choice2,
}

#[derive(Deserialize)]
pub(super) enum BoardExportDocumentFormat {
    #[serde(rename = "wekan-board-1.0.0")]
    Choice0,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportAttachment {
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "cardId", default, deserialize_with = "optional")]
    _card_id: Option<String>,
    #[serde(rename = "source", default, deserialize_with = "optional")]
    _source: Option<String>,
    #[serde(rename = "name", default, deserialize_with = "optional")]
    _name: Option<String>,
    #[serde(rename = "type", default, deserialize_with = "optional")]
    _type: Option<String>,
    #[serde(rename = "file", default, deserialize_with = "optional")]
    _file: Option<Option<Base64>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportList {
    #[serde(rename = "title")]
    _title: String,
    #[serde(rename = "starred", default, deserialize_with = "optional")]
    _starred: Option<Option<bool>>,
    #[serde(rename = "archived")]
    _archived: bool,
    #[serde(rename = "archivedAt", default, deserialize_with = "optional")]
    _archived_at: Option<Option<Date>>,
    #[serde(rename = "deletedAt", default, deserialize_with = "optional")]
    _deleted_at: Option<Option<Date>>,
    #[serde(rename = "deletedBy", default, deserialize_with = "optional")]
    _deleted_by: Option<Option<String>>,
    #[serde(rename = "deleteBatchId", default, deserialize_with = "optional")]
    _delete_batch_id: Option<Option<String>>,
    #[serde(rename = "swimlaneId", default, deserialize_with = "optional")]
    _swimlane_id: Option<Option<String>>,
    #[serde(rename = "createdAt")]
    _created_at: Date,
    #[serde(rename = "sort", default, deserialize_with = "optional")]
    _sort: Option<Option<serde_json::Number>>,
    #[serde(rename = "updatedAt", default, deserialize_with = "optional")]
    _updated_at: Option<Option<Date>>,
    #[serde(rename = "modifiedAt")]
    _modified_at: Date,
    #[serde(rename = "wipLimit", default, deserialize_with = "optional")]
    _wip_limit: Option<ListsWiplimit>,
    #[serde(rename = "color", default, deserialize_with = "optional")]
    _color: Option<Option<String>>,
    #[serde(rename = "type")]
    _type: String,
    #[serde(rename = "width", default, deserialize_with = "optional")]
    _width: Option<Option<serde_json::Number>>,
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "_updatedAt", default, deserialize_with = "optional")]
    __updated_at: Option<Option<Date>>,
    #[serde(rename = "watchers", default, deserialize_with = "optional")]
    _watchers: Option<DiscardVec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportSwimlane {
    #[serde(rename = "title")]
    _title: String,
    #[serde(rename = "archived")]
    _archived: bool,
    #[serde(rename = "archivedAt", default, deserialize_with = "optional")]
    _archived_at: Option<Option<Date>>,
    #[serde(rename = "createdAt")]
    _created_at: Date,
    #[serde(rename = "sort", default, deserialize_with = "optional")]
    _sort: Option<Option<serde_json::Number>>,
    #[serde(rename = "color", default, deserialize_with = "optional")]
    _color: Option<Option<String>>,
    #[serde(rename = "updatedAt", default, deserialize_with = "optional")]
    _updated_at: Option<Option<Date>>,
    #[serde(rename = "modifiedAt")]
    _modified_at: Date,
    #[serde(rename = "type")]
    _type: String,
    #[serde(rename = "height", default, deserialize_with = "optional")]
    _height: Option<Option<serde_json::Number>>,
    #[serde(rename = "_id")]
    __id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportCustomField {
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "type")]
    _type: ExportCustomFieldType,
    #[serde(rename = "settings")]
    _settings: CustomFieldsSettings,
    #[serde(rename = "showOnCard")]
    _show_on_card: bool,
    #[serde(rename = "automaticallyOnCard")]
    _automatically_on_card: bool,
    #[serde(rename = "alwaysOnCard")]
    _always_on_card: bool,
    #[serde(rename = "showLabelOnMiniCard")]
    _show_label_on_mini_card: bool,
    #[serde(rename = "showSumAtTopOfList")]
    _show_sum_at_top_of_list: bool,
    #[serde(rename = "createdAt", default, deserialize_with = "optional")]
    _created_at: Option<Option<Date>>,
    #[serde(rename = "modifiedAt")]
    _modified_at: Date,
    #[serde(rename = "_id")]
    __id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportCard {
    #[serde(rename = "title", default, deserialize_with = "optional")]
    _title: Option<Option<String>>,
    #[serde(rename = "archived")]
    _archived: bool,
    #[serde(rename = "archivedAt", default, deserialize_with = "optional")]
    _archived_at: Option<Option<Date>>,
    #[serde(rename = "deletedAt", default, deserialize_with = "optional")]
    _deleted_at: Option<Option<Date>>,
    #[serde(rename = "deletedBy", default, deserialize_with = "optional")]
    _deleted_by: Option<Option<String>>,
    #[serde(rename = "deleteBatchId", default, deserialize_with = "optional")]
    _delete_batch_id: Option<Option<String>>,
    #[serde(rename = "parentId", default, deserialize_with = "optional")]
    _parent_id: Option<Option<String>>,
    #[serde(rename = "listId", default, deserialize_with = "optional")]
    _list_id: Option<Option<String>>,
    #[serde(rename = "swimlaneId")]
    _swimlane_id: String,
    #[serde(rename = "coverId", default, deserialize_with = "optional")]
    _cover_id: Option<Option<String>>,
    #[serde(rename = "color", default, deserialize_with = "optional")]
    _color: Option<Option<String>>,
    #[serde(rename = "createdAt")]
    _created_at: Date,
    #[serde(rename = "modifiedAt")]
    _modified_at: Date,
    #[serde(rename = "customFields", default, deserialize_with = "optional")]
    _custom_fields: Option<Option<DiscardVec<CardsCustomfields>>>,
    #[serde(rename = "dateLastActivity")]
    _date_last_activity: Date,
    #[serde(rename = "description", default, deserialize_with = "optional")]
    _description: Option<Option<String>>,
    #[serde(rename = "requestedBy", default, deserialize_with = "optional")]
    _requested_by: Option<Option<String>>,
    #[serde(rename = "assignedBy", default, deserialize_with = "optional")]
    _assigned_by: Option<Option<String>>,
    #[serde(rename = "labelIds", default, deserialize_with = "optional")]
    _label_ids: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "members", default, deserialize_with = "optional")]
    _members: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "assignees", default, deserialize_with = "optional")]
    _assignees: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "requesters", default, deserialize_with = "optional")]
    _requesters: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "assigners", default, deserialize_with = "optional")]
    _assigners: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "watchers", default, deserialize_with = "optional")]
    _watchers: Option<DiscardVec<String>>,
    #[serde(rename = "receivedAt", default, deserialize_with = "optional")]
    _received_at: Option<Option<Date>>,
    #[serde(rename = "startAt", default, deserialize_with = "optional")]
    _start_at: Option<Option<Date>>,
    #[serde(rename = "dueAt", default, deserialize_with = "optional")]
    _due_at: Option<Option<Date>>,
    #[serde(rename = "endAt", default, deserialize_with = "optional")]
    _end_at: Option<Option<Date>>,
    #[serde(rename = "dueComplete", default, deserialize_with = "optional")]
    _due_complete: Option<Option<bool>>,
    #[serde(rename = "stickers", default, deserialize_with = "optional")]
    _stickers: Option<Option<DiscardVec<CardsStickers>>>,
    #[serde(rename = "locationName", default, deserialize_with = "optional")]
    _location_name: Option<Option<String>>,
    #[serde(rename = "locationAddress", default, deserialize_with = "optional")]
    _location_address: Option<Option<String>>,
    #[serde(rename = "locationLatitude", default, deserialize_with = "optional")]
    _location_latitude: Option<Option<serde_json::Number>>,
    #[serde(rename = "locationLongitude", default, deserialize_with = "optional")]
    _location_longitude: Option<Option<serde_json::Number>>,
    #[serde(rename = "locations", default, deserialize_with = "optional")]
    _locations: Option<Option<DiscardVec<CardsLocations>>>,
    #[serde(rename = "spentTime", default, deserialize_with = "optional")]
    _spent_time: Option<Option<serde_json::Number>>,
    #[serde(rename = "isOvertime", default, deserialize_with = "optional")]
    _is_overtime: Option<Option<bool>>,
    #[serde(rename = "userId")]
    _user_id: String,
    #[serde(rename = "sort", default, deserialize_with = "optional")]
    _sort: Option<Option<serde_json::Number>>,
    #[serde(rename = "subtaskSort", default, deserialize_with = "optional")]
    _subtask_sort: Option<Option<serde_json::Number>>,
    #[serde(rename = "type")]
    _type: ExportCardType,
    #[serde(rename = "linkedId", default, deserialize_with = "optional")]
    _linked_id: Option<Option<String>>,
    #[serde(rename = "cardDependencies", default, deserialize_with = "optional")]
    _card_dependencies: Option<Option<DiscardVec<CardsCarddependencies>>>,
    #[serde(rename = "vote", default, deserialize_with = "optional")]
    _vote: Option<CardsVote>,
    #[serde(rename = "poker", default, deserialize_with = "optional")]
    _poker: Option<CardsPoker>,
    #[serde(rename = "targetId_gantt", default, deserialize_with = "optional")]
    _target_id_gantt: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "linkType_gantt", default, deserialize_with = "optional")]
    _link_type_gantt: Option<Option<DiscardVec<Option<serde_json::Number>>>>,
    #[serde(rename = "linkId_gantt", default, deserialize_with = "optional")]
    _link_id_gantt: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "cardNumber", default, deserialize_with = "optional")]
    _card_number: Option<Option<serde_json::Number>>,
    #[serde(rename = "showActivities")]
    _show_activities: bool,
    #[serde(rename = "showListOnMinicard", default, deserialize_with = "optional")]
    _show_list_on_minicard: Option<Option<bool>>,
    #[serde(
        rename = "showChecklistAtMinicard",
        default,
        deserialize_with = "optional"
    )]
    _show_checklist_at_minicard: Option<Option<bool>>,
    #[serde(
        rename = "hideFinishedChecklistIfItemsAreHidden",
        default,
        deserialize_with = "optional"
    )]
    _hide_finished_checklist_if_items_are_hidden: Option<Option<bool>>,
    #[serde(rename = "_id")]
    __id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportComment {
    #[serde(rename = "cardId")]
    _card_id: String,
    #[serde(rename = "text")]
    _text: String,
    #[serde(rename = "parentId", default, deserialize_with = "optional")]
    _parent_id: Option<Option<String>>,
    #[serde(rename = "createdAt")]
    _created_at: Date,
    #[serde(rename = "modifiedAt")]
    _modified_at: Date,
    #[serde(rename = "userId")]
    _user_id: String,
    #[serde(rename = "_id")]
    __id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportActivity {
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "userId", default, deserialize_with = "optional")]
    _user_id: Option<String>,
    #[serde(rename = "createdAt", default, deserialize_with = "optional")]
    _created_at: Option<Date>,
    #[serde(rename = "modifiedAt", default, deserialize_with = "optional")]
    _modified_at: Option<Date>,
    #[serde(rename = "activityType", default, deserialize_with = "optional")]
    _activity_type: Option<String>,
    #[serde(rename = "cardId", default, deserialize_with = "optional")]
    _card_id: Option<String>,
    #[serde(rename = "listId", default, deserialize_with = "optional")]
    _list_id: Option<String>,
    #[serde(rename = "swimlaneId", default, deserialize_with = "optional")]
    _swimlane_id: Option<String>,
    #[serde(rename = "memberId", default, deserialize_with = "optional")]
    _member_id: Option<String>,
    #[serde(rename = "type", default, deserialize_with = "optional")]
    _type: Option<Option<String>>,
    #[serde(rename = "title", default, deserialize_with = "optional")]
    _title: Option<Option<String>>,
    #[serde(rename = "activityTypeId", default, deserialize_with = "optional")]
    _activity_type_id: Option<Option<String>>,
    #[serde(rename = "assigneeId", default, deserialize_with = "optional")]
    _assignee_id: Option<Option<String>>,
    #[serde(rename = "attachmentId", default, deserialize_with = "optional")]
    _attachment_id: Option<Option<String>>,
    #[serde(rename = "attachmentName", default, deserialize_with = "optional")]
    _attachment_name: Option<Option<String>>,
    #[serde(rename = "boardName", default, deserialize_with = "optional")]
    _board_name: Option<Option<String>>,
    #[serde(rename = "oldBoardName", default, deserialize_with = "optional")]
    _old_board_name: Option<Option<String>>,
    #[serde(rename = "cardTitle", default, deserialize_with = "optional")]
    _card_title: Option<Option<String>>,
    #[serde(rename = "checklistId", default, deserialize_with = "optional")]
    _checklist_id: Option<Option<String>>,
    #[serde(rename = "checklistItemId", default, deserialize_with = "optional")]
    _checklist_item_id: Option<Option<String>>,
    #[serde(rename = "checklistItemName", default, deserialize_with = "optional")]
    _checklist_item_name: Option<Option<String>>,
    #[serde(rename = "checklistName", default, deserialize_with = "optional")]
    _checklist_name: Option<Option<String>>,
    #[serde(rename = "commentId", default, deserialize_with = "optional")]
    _comment_id: Option<Option<String>>,
    #[serde(rename = "commentText", default, deserialize_with = "optional")]
    _comment_text: Option<Option<String>>,
    #[serde(rename = "customFieldId", default, deserialize_with = "optional")]
    _custom_field_id: Option<Option<String>>,
    #[serde(rename = "labelId", default, deserialize_with = "optional")]
    _label_id: Option<Option<String>>,
    #[serde(rename = "listName", default, deserialize_with = "optional")]
    _list_name: Option<Option<String>>,
    #[serde(rename = "oldBoardId", default, deserialize_with = "optional")]
    _old_board_id: Option<Option<String>>,
    #[serde(rename = "oldListId", default, deserialize_with = "optional")]
    _old_list_id: Option<Option<String>>,
    #[serde(rename = "oldSwimlaneId", default, deserialize_with = "optional")]
    _old_swimlane_id: Option<Option<String>>,
    #[serde(rename = "swimlaneName", default, deserialize_with = "optional")]
    _swimlane_name: Option<Option<String>>,
    #[serde(rename = "timeKey", default, deserialize_with = "optional")]
    _time_key: Option<Option<String>>,
    #[serde(rename = "timeValue", default, deserialize_with = "optional")]
    _time_value: Option<Option<String>>,
    #[serde(rename = "timeOldValue", default, deserialize_with = "optional")]
    _time_old_value: Option<Option<String>>,
    #[serde(rename = "username", default, deserialize_with = "optional")]
    _username: Option<Option<String>>,
    #[serde(rename = "value", default, deserialize_with = "optional")]
    _value: Option<Option<CardCustomFieldValue>>,
    #[serde(rename = "oldValue", default, deserialize_with = "optional")]
    _old_value: Option<Option<CardCustomFieldValue>>,
    #[serde(rename = "customFieldValue", default, deserialize_with = "optional")]
    _custom_field_value: Option<Option<CardCustomFieldValue>>,
    #[serde(rename = "source", default, deserialize_with = "optional")]
    _source: Option<ExportActivitySource>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportActivitySource {
    #[serde(rename = "id", default, deserialize_with = "optional")]
    _id: Option<String>,
    #[serde(rename = "system")]
    _system: String,
    #[serde(rename = "url", default, deserialize_with = "optional")]
    _url: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Checklists {
    #[serde(rename = "cardId")]
    _card_id: String,
    #[serde(rename = "boardId", default, deserialize_with = "optional")]
    _board_id: Option<Option<String>>,
    #[serde(rename = "userId", default, deserialize_with = "optional")]
    _user_id: Option<Option<String>>,
    #[serde(rename = "title")]
    _title: String,
    #[serde(rename = "finishedAt", default, deserialize_with = "optional")]
    _finished_at: Option<Option<Date>>,
    #[serde(rename = "createdAt")]
    _created_at: Date,
    #[serde(rename = "modifiedAt")]
    _modified_at: Date,
    #[serde(rename = "sort")]
    _sort: serde_json::Number,
    #[serde(
        rename = "hideCheckedChecklistItems",
        default,
        deserialize_with = "optional"
    )]
    _hide_checked_checklist_items: Option<Option<bool>>,
    #[serde(
        rename = "hideAllChecklistItems",
        default,
        deserialize_with = "optional"
    )]
    _hide_all_checklist_items: Option<Option<bool>>,
    #[serde(
        rename = "showChecklistAtMinicard",
        default,
        deserialize_with = "optional"
    )]
    _show_checklist_at_minicard: Option<Option<bool>>,
    #[serde(rename = "_id")]
    __id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ChecklistItems {
    #[serde(rename = "title")]
    _title: String,
    #[serde(rename = "sort")]
    _sort: serde_json::Number,
    #[serde(rename = "isFinished")]
    _is_finished: bool,
    #[serde(rename = "checklistId")]
    _checklist_id: String,
    #[serde(rename = "cardId")]
    _card_id: String,
    #[serde(rename = "boardId", default, deserialize_with = "optional")]
    _board_id: Option<Option<String>>,
    #[serde(rename = "createdAt", default, deserialize_with = "optional")]
    _created_at: Option<Option<Date>>,
    #[serde(rename = "modifiedAt")]
    _modified_at: Date,
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "userId", default, deserialize_with = "optional")]
    _user_id: Option<Option<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Cards {
    #[serde(rename = "title", default, deserialize_with = "optional")]
    _title: Option<Option<String>>,
    #[serde(rename = "archived")]
    _archived: bool,
    #[serde(rename = "archivedAt", default, deserialize_with = "optional")]
    _archived_at: Option<Option<String>>,
    #[serde(rename = "deletedAt", default, deserialize_with = "optional")]
    _deleted_at: Option<Option<String>>,
    #[serde(rename = "deletedBy", default, deserialize_with = "optional")]
    _deleted_by: Option<Option<String>>,
    #[serde(rename = "deleteBatchId", default, deserialize_with = "optional")]
    _delete_batch_id: Option<Option<String>>,
    #[serde(rename = "parentId", default, deserialize_with = "optional")]
    _parent_id: Option<Option<String>>,
    #[serde(rename = "listId", default, deserialize_with = "optional")]
    _list_id: Option<Option<String>>,
    #[serde(rename = "swimlaneId")]
    _swimlane_id: String,
    #[serde(rename = "boardId", default, deserialize_with = "optional")]
    _board_id: Option<Option<String>>,
    #[serde(rename = "coverId", default, deserialize_with = "optional")]
    _cover_id: Option<Option<String>>,
    #[serde(rename = "color", default, deserialize_with = "optional")]
    _color: Option<Option<String>>,
    #[serde(rename = "createdAt")]
    _created_at: String,
    #[serde(rename = "modifiedAt")]
    _modified_at: String,
    #[serde(rename = "customFields", default, deserialize_with = "optional")]
    _custom_fields: Option<Option<DiscardVec<CardsCustomfields>>>,
    #[serde(rename = "dateLastActivity")]
    _date_last_activity: String,
    #[serde(rename = "description", default, deserialize_with = "optional")]
    _description: Option<Option<String>>,
    #[serde(rename = "requestedBy", default, deserialize_with = "optional")]
    _requested_by: Option<Option<String>>,
    #[serde(rename = "assignedBy", default, deserialize_with = "optional")]
    _assigned_by: Option<Option<String>>,
    #[serde(rename = "labelIds", default, deserialize_with = "optional")]
    _label_ids: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "members", default, deserialize_with = "optional")]
    _members: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "assignees", default, deserialize_with = "optional")]
    _assignees: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "requesters", default, deserialize_with = "optional")]
    _requesters: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "assigners", default, deserialize_with = "optional")]
    _assigners: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "watchers", default, deserialize_with = "optional")]
    _watchers: Option<DiscardVec<String>>,
    #[serde(rename = "receivedAt", default, deserialize_with = "optional")]
    _received_at: Option<Option<String>>,
    #[serde(rename = "startAt", default, deserialize_with = "optional")]
    _start_at: Option<Option<String>>,
    #[serde(rename = "dueAt", default, deserialize_with = "optional")]
    _due_at: Option<Option<String>>,
    #[serde(rename = "endAt", default, deserialize_with = "optional")]
    _end_at: Option<Option<String>>,
    #[serde(rename = "dueComplete", default, deserialize_with = "optional")]
    _due_complete: Option<Option<bool>>,
    #[serde(rename = "stickers", default, deserialize_with = "optional")]
    _stickers: Option<Option<DiscardVec<CardsStickers>>>,
    #[serde(rename = "locationName", default, deserialize_with = "optional")]
    _location_name: Option<Option<String>>,
    #[serde(rename = "locationAddress", default, deserialize_with = "optional")]
    _location_address: Option<Option<String>>,
    #[serde(rename = "locationLatitude", default, deserialize_with = "optional")]
    _location_latitude: Option<Option<serde_json::Number>>,
    #[serde(rename = "locationLongitude", default, deserialize_with = "optional")]
    _location_longitude: Option<Option<serde_json::Number>>,
    #[serde(rename = "locations", default, deserialize_with = "optional")]
    _locations: Option<Option<DiscardVec<CardsLocations>>>,
    #[serde(rename = "spentTime", default, deserialize_with = "optional")]
    _spent_time: Option<Option<serde_json::Number>>,
    #[serde(rename = "isOvertime", default, deserialize_with = "optional")]
    _is_overtime: Option<Option<bool>>,
    #[serde(rename = "userId")]
    _user_id: String,
    #[serde(rename = "sort", default, deserialize_with = "optional")]
    _sort: Option<Option<serde_json::Number>>,
    #[serde(rename = "subtaskSort", default, deserialize_with = "optional")]
    _subtask_sort: Option<Option<serde_json::Number>>,
    #[serde(rename = "type")]
    _type: CardsType,
    #[serde(rename = "linkedId", default, deserialize_with = "optional")]
    _linked_id: Option<Option<String>>,
    #[serde(rename = "cardDependencies", default, deserialize_with = "optional")]
    _card_dependencies: Option<Option<DiscardVec<CardsCarddependencies>>>,
    #[serde(rename = "vote", default, deserialize_with = "optional")]
    _vote: Option<CardsVote>,
    #[serde(rename = "poker", default, deserialize_with = "optional")]
    _poker: Option<CardsPoker>,
    #[serde(rename = "targetId_gantt", default, deserialize_with = "optional")]
    _target_id_gantt: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "linkType_gantt", default, deserialize_with = "optional")]
    _link_type_gantt: Option<Option<DiscardVec<Option<serde_json::Number>>>>,
    #[serde(rename = "linkId_gantt", default, deserialize_with = "optional")]
    _link_id_gantt: Option<Option<DiscardVec<Option<String>>>>,
    #[serde(rename = "cardNumber", default, deserialize_with = "optional")]
    _card_number: Option<Option<serde_json::Number>>,
    #[serde(rename = "showActivities")]
    _show_activities: bool,
    #[serde(rename = "showListOnMinicard", default, deserialize_with = "optional")]
    _show_list_on_minicard: Option<Option<bool>>,
    #[serde(
        rename = "showChecklistAtMinicard",
        default,
        deserialize_with = "optional"
    )]
    _show_checklist_at_minicard: Option<Option<bool>>,
    #[serde(
        rename = "hideFinishedChecklistIfItemsAreHidden",
        default,
        deserialize_with = "optional"
    )]
    _hide_finished_checklist_if_items_are_hidden: Option<Option<bool>>,
    #[serde(rename = "_id")]
    __id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportRule {
    #[serde(rename = "title")]
    _title: String,
    #[serde(rename = "triggerId")]
    _trigger_id: String,
    #[serde(rename = "actionId")]
    _action_id: String,
    #[serde(rename = "buttonType", default, deserialize_with = "optional")]
    _button_type: Option<Option<ExportRuleButtonType>>,
    #[serde(rename = "buttonLabel", default, deserialize_with = "optional")]
    _button_label: Option<Option<String>>,
    #[serde(rename = "createdAt", default, deserialize_with = "optional")]
    _created_at: Option<Option<Date>>,
    #[serde(rename = "modifiedAt")]
    _modified_at: Date,
    #[serde(rename = "_id")]
    __id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportTrigger {
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "createdAt", default, deserialize_with = "optional")]
    _created_at: Option<Date>,
    #[serde(rename = "updatedAt", default, deserialize_with = "optional")]
    _updated_at: Option<Date>,
    #[serde(rename = "userId", default, deserialize_with = "optional")]
    _user_id: Option<Option<String>>,
    #[serde(rename = "activityType", default, deserialize_with = "optional")]
    _activity_type: Option<Option<String>>,
    #[serde(rename = "desc", default, deserialize_with = "optional")]
    _desc: Option<Option<String>>,
    #[serde(rename = "description", default, deserialize_with = "optional")]
    _description: Option<Option<String>>,
    #[serde(rename = "username", default, deserialize_with = "optional")]
    _username: Option<Option<String>>,
    #[serde(rename = "listName", default, deserialize_with = "optional")]
    _list_name: Option<Option<String>>,
    #[serde(rename = "oldListName", default, deserialize_with = "optional")]
    _old_list_name: Option<Option<String>>,
    #[serde(rename = "swimlaneName", default, deserialize_with = "optional")]
    _swimlane_name: Option<Option<String>>,
    #[serde(rename = "cardTitle", default, deserialize_with = "optional")]
    _card_title: Option<Option<String>>,
    #[serde(rename = "checklistName", default, deserialize_with = "optional")]
    _checklist_name: Option<Option<String>>,
    #[serde(rename = "checklistItemName", default, deserialize_with = "optional")]
    _checklist_item_name: Option<Option<String>>,
    #[serde(rename = "labelId", default, deserialize_with = "optional")]
    _label_id: Option<Option<String>>,
    #[serde(rename = "attachmentName", default, deserialize_with = "optional")]
    _attachment_name: Option<Option<String>>,
    #[serde(rename = "scheduleKind", default, deserialize_with = "optional")]
    _schedule_kind: Option<Option<String>>,
    #[serde(rename = "scheduleType", default, deserialize_with = "optional")]
    _schedule_type: Option<Option<String>>,
    #[serde(rename = "atTime", default, deserialize_with = "optional")]
    _at_time: Option<Option<String>>,
    #[serde(rename = "weekday", default, deserialize_with = "optional")]
    _weekday: Option<Option<ExportTriggerWeekday>>,
    #[serde(rename = "dayOfMonth", default, deserialize_with = "optional")]
    _day_of_month: Option<Option<ExportTriggerDayOfMonth>>,
    #[serde(rename = "onDate", default, deserialize_with = "optional")]
    _on_date: Option<Option<String>>,
    #[serde(rename = "dueCondition", default, deserialize_with = "optional")]
    _due_condition: Option<Option<String>>,
    #[serde(rename = "days", default, deserialize_with = "optional")]
    _days: Option<Option<ExportTriggerDays>>,
    #[serde(rename = "lastRunKey", default, deserialize_with = "optional")]
    _last_run_key: Option<Option<String>>,
    #[serde(rename = "lastRunAt", default, deserialize_with = "optional")]
    _last_run_at: Option<Option<Date>>,
    #[serde(rename = "buttonType", default, deserialize_with = "optional")]
    _button_type: Option<Option<String>>,
    #[serde(rename = "buttonLabel", default, deserialize_with = "optional")]
    _button_label: Option<Option<String>>,
    #[serde(rename = "modifiedAt", default, deserialize_with = "optional")]
    _modified_at: Option<Option<Date>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportAction {
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "createdAt", default, deserialize_with = "optional")]
    _created_at: Option<Date>,
    #[serde(rename = "modifiedAt", default, deserialize_with = "optional")]
    _modified_at: Option<Date>,
    #[serde(rename = "userId", default, deserialize_with = "optional")]
    _user_id: Option<Option<String>>,
    #[serde(rename = "actionType", default, deserialize_with = "optional")]
    _action_type: Option<Option<String>>,
    #[serde(rename = "desc", default, deserialize_with = "optional")]
    _desc: Option<Option<String>>,
    #[serde(rename = "description", default, deserialize_with = "optional")]
    _description: Option<Option<String>>,
    #[serde(rename = "listName", default, deserialize_with = "optional")]
    _list_name: Option<Option<String>>,
    #[serde(rename = "swimlaneName", default, deserialize_with = "optional")]
    _swimlane_name: Option<Option<String>>,
    #[serde(rename = "cardName", default, deserialize_with = "optional")]
    _card_name: Option<Option<String>>,
    #[serde(rename = "fromListName", default, deserialize_with = "optional")]
    _from_list_name: Option<Option<String>>,
    #[serde(rename = "checklistName", default, deserialize_with = "optional")]
    _checklist_name: Option<Option<String>>,
    #[serde(rename = "checkItemName", default, deserialize_with = "optional")]
    _check_item_name: Option<Option<String>>,
    #[serde(rename = "checklistItems", default, deserialize_with = "optional")]
    _checklist_items: Option<Option<String>>,
    #[serde(rename = "dateField", default, deserialize_with = "optional")]
    _date_field: Option<Option<String>>,
    #[serde(rename = "unit", default, deserialize_with = "optional")]
    _unit: Option<Option<String>>,
    #[serde(rename = "emailTo", default, deserialize_with = "optional")]
    _email_to: Option<Option<String>>,
    #[serde(rename = "emailSubject", default, deserialize_with = "optional")]
    _email_subject: Option<Option<String>>,
    #[serde(rename = "emailMsg", default, deserialize_with = "optional")]
    _email_msg: Option<Option<String>>,
    #[serde(rename = "selectedColor", default, deserialize_with = "optional")]
    _selected_color: Option<Option<String>>,
    #[serde(rename = "labelId", default, deserialize_with = "optional")]
    _label_id: Option<Option<String>>,
    #[serde(rename = "username", default, deserialize_with = "optional")]
    _username: Option<Option<String>>,
    #[serde(rename = "sortField", default, deserialize_with = "optional")]
    _sort_field: Option<Option<String>>,
    #[serde(rename = "days", default, deserialize_with = "optional")]
    _days: Option<Option<ExportActionDays>>,
    #[serde(rename = "updatedAt", default, deserialize_with = "optional")]
    _updated_at: Option<Option<Date>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportUser {
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "username", default, deserialize_with = "optional")]
    _username: Option<String>,
    #[serde(rename = "profile", default, deserialize_with = "optional")]
    _profile: Option<ExportUserProfile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KanboardExportBoard {
    #[serde(rename = "name")]
    _name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KanboardExportColumnsItem {
    #[serde(rename = "title")]
    _title: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KanboardExportSwimlanesItem {
    #[serde(rename = "name")]
    _name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KanboardExportTasksItem {
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    _title: Option<String>,
    #[serde(rename = "description")]
    _description: String,
    #[serde(rename = "column_name")]
    _column_name: String,
    #[serde(rename = "swimlane_name")]
    _swimlane_name: String,
    #[serde(rename = "date_due", default, deserialize_with = "optional")]
    _date_due: Option<Date>,
    #[serde(rename = "tags")]
    _tags: DiscardVec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SingleAttachmentExportChoice0 {
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "cardId", default, deserialize_with = "optional")]
    _card_id: Option<String>,
    #[serde(rename = "source", default, deserialize_with = "optional")]
    _source: Option<String>,
    #[serde(rename = "name", default, deserialize_with = "optional")]
    _name: Option<String>,
    #[serde(rename = "type", default, deserialize_with = "optional")]
    _type: Option<String>,
    #[serde(rename = "file", deserialize_with = "required_nullable")]
    _file: Option<Base64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SingleAttachmentExportChoice1 {}

#[derive(Deserialize)]
pub(super) enum GithubIssueExportState {
    #[serde(rename = "open")]
    Choice0,
    #[serde(rename = "closed")]
    Choice1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GithubIssueExportLabelsItem {
    #[serde(rename = "name")]
    _name: String,
}

#[derive(Deserialize)]
pub(super) enum GitlabIssueExportState {
    #[serde(rename = "opened")]
    Choice0,
    #[serde(rename = "closed")]
    Choice1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeckExportStacksItem {
    #[serde(rename = "title")]
    _title: String,
    #[serde(rename = "cards")]
    _cards: DiscardVec<DeckExportStacksItemCardsItem>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OpenprojectExportEmbedded {
    #[serde(rename = "elements")]
    _elements: DiscardVec<OpenprojectExportEmbeddedElementsItem>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TrelloExportPrefs {
    #[serde(rename = "background")]
    _background: TrelloExportPrefsBackground,
    #[serde(rename = "permissionLevel")]
    _permission_level: TrelloExportPrefsPermissionLevel,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TrelloExportLabelsItem {
    #[serde(rename = "id")]
    _id: String,
    #[serde(rename = "name", default, deserialize_with = "optional")]
    _name: Option<Option<String>>,
    #[serde(rename = "color")]
    _color: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TrelloExportListsItem {
    #[serde(rename = "id")]
    _id: String,
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "closed")]
    _closed: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TrelloExportCardsItem {
    #[serde(rename = "id")]
    _id: String,
    #[serde(rename = "name", deserialize_with = "required_nullable")]
    _name: Option<String>,
    #[serde(rename = "desc")]
    _desc: String,
    #[serde(rename = "idList", deserialize_with = "required_nullable")]
    _id_list: Option<String>,
    #[serde(rename = "due", deserialize_with = "required_nullable")]
    _due: Option<Date>,
    #[serde(rename = "closed")]
    _closed: bool,
    #[serde(rename = "idLabels")]
    _id_labels: DiscardVec<Option<String>>,
    #[serde(rename = "idMembers")]
    _id_members: DiscardVec<String>,
    #[serde(rename = "idChecklists")]
    _id_checklists: DiscardVec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JiraExportBoard {
    #[serde(rename = "name")]
    _name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JiraExportIssuesItem {
    #[serde(rename = "key")]
    _key: String,
    #[serde(rename = "fields")]
    _fields: JiraExportIssuesItemFields,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AsanaExportDataItem {
    #[serde(rename = "name", deserialize_with = "required_nullable")]
    _name: Option<String>,
    #[serde(rename = "notes")]
    _notes: String,
    #[serde(rename = "completed")]
    _completed: bool,
    #[serde(rename = "due_on", default, deserialize_with = "optional")]
    _due_on: Option<Date>,
    #[serde(rename = "memberships")]
    _memberships: DiscardVec<AsanaExportDataItemMembershipsItem>,
    #[serde(rename = "tags")]
    _tags: DiscardVec<AsanaExportDataItemTagsItem>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ZenkitExportStagesItem {
    #[serde(rename = "name")]
    _name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ZenkitExportItemsItem {
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    _title: Option<String>,
    #[serde(rename = "description")]
    _description: String,
    #[serde(rename = "stage_name")]
    _stage_name: String,
    #[serde(rename = "due", default, deserialize_with = "optional")]
    _due: Option<Date>,
    #[serde(rename = "tags")]
    _tags: DiscardVec<String>,
}

#[derive(Deserialize)]
pub(super) enum BoardWatcherLevel {
    #[serde(rename = "watching")]
    Choice0,
    #[serde(rename = "tracking")]
    Choice1,
    #[serde(rename = "muted")]
    Choice2,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ListsWiplimit {
    #[serde(rename = "value")]
    _value: serde_json::Number,
    #[serde(rename = "enabled")]
    _enabled: bool,
    #[serde(rename = "soft")]
    _soft: bool,
}

#[derive(Deserialize)]
pub(super) enum ExportCustomFieldType {
    #[serde(rename = "text")]
    Choice0,
    #[serde(rename = "number")]
    Choice1,
    #[serde(rename = "date")]
    Choice2,
    #[serde(rename = "dropdown")]
    Choice3,
    #[serde(rename = "checkbox")]
    Choice4,
    #[serde(rename = "currency")]
    Choice5,
    #[serde(rename = "stringtemplate")]
    Choice6,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CustomFieldsSettings {
    #[serde(rename = "currencyCode", default, deserialize_with = "optional")]
    _currency_code: Option<Option<String>>,
    #[serde(rename = "dropdownItems", default, deserialize_with = "optional")]
    _dropdown_items: Option<Option<DiscardVec<CustomFieldsSettingsDropdownitems>>>,
    #[serde(
        rename = "stringtemplateFormat",
        default,
        deserialize_with = "optional"
    )]
    _stringtemplate_format: Option<Option<String>>,
    #[serde(
        rename = "stringtemplateSeparator",
        default,
        deserialize_with = "optional"
    )]
    _stringtemplate_separator: Option<Option<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CardsCustomfields {
    #[serde(rename = "_id", default, deserialize_with = "optional")]
    __id: Option<Option<String>>,
    #[serde(rename = "value", default, deserialize_with = "optional")]
    _value: Option<Option<CardCustomFieldValue>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CardsStickers {
    #[serde(rename = "icon", default, deserialize_with = "optional")]
    _icon: Option<String>,
    #[serde(rename = "name", default, deserialize_with = "optional")]
    _name: Option<String>,
    #[serde(rename = "highlight", default, deserialize_with = "optional")]
    _highlight: Option<CardsStickersHighlight>,
    #[serde(rename = "position", default, deserialize_with = "optional")]
    _position: Option<serde_json::Number>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CardsLocations {
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "name", default, deserialize_with = "optional")]
    _name: Option<String>,
    #[serde(rename = "address", default, deserialize_with = "optional")]
    _address: Option<String>,
    #[serde(rename = "latitude", default, deserialize_with = "optional")]
    _latitude: Option<serde_json::Number>,
    #[serde(rename = "longitude", default, deserialize_with = "optional")]
    _longitude: Option<serde_json::Number>,
}

#[derive(Deserialize)]
pub(super) enum ExportCardType {
    #[serde(rename = "cardType-card")]
    Choice0,
    #[serde(rename = "cardType-linkedCard")]
    Choice1,
    #[serde(rename = "cardType-linkedBoard")]
    Choice2,
    #[serde(rename = "template-card")]
    Choice3,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CardsCarddependencies {
    #[serde(rename = "cardId")]
    _card_id: String,
    #[serde(rename = "type", default, deserialize_with = "optional")]
    _type: Option<Option<String>>,
    #[serde(rename = "color", default, deserialize_with = "optional")]
    _color: Option<Option<String>>,
    #[serde(rename = "icon", default, deserialize_with = "optional")]
    _icon: Option<Option<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CardsVote {
    #[serde(rename = "question")]
    _question: String,
    #[serde(rename = "positive", default, deserialize_with = "optional")]
    _positive: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "negative", default, deserialize_with = "optional")]
    _negative: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "end", default, deserialize_with = "optional")]
    _end: Option<Option<String>>,
    #[serde(rename = "public")]
    _public: bool,
    #[serde(rename = "allowNonBoardMembers")]
    _allow_non_board_members: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CardsPoker {
    #[serde(rename = "question", default, deserialize_with = "optional")]
    _question: Option<bool>,
    #[serde(rename = "one", default, deserialize_with = "optional")]
    _one: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "two", default, deserialize_with = "optional")]
    _two: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "three", default, deserialize_with = "optional")]
    _three: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "five", default, deserialize_with = "optional")]
    _five: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "eight", default, deserialize_with = "optional")]
    _eight: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "thirteen", default, deserialize_with = "optional")]
    _thirteen: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "twenty", default, deserialize_with = "optional")]
    _twenty: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "forty", default, deserialize_with = "optional")]
    _forty: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "oneHundred", default, deserialize_with = "optional")]
    _one_hundred: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "unsure", default, deserialize_with = "optional")]
    _unsure: Option<DiscardVec<Option<String>>>,
    #[serde(rename = "end", default, deserialize_with = "optional")]
    _end: Option<Option<String>>,
    #[serde(
        rename = "allowNonBoardMembers",
        default,
        deserialize_with = "optional"
    )]
    _allow_non_board_members: Option<bool>,
    #[serde(rename = "estimation", default, deserialize_with = "optional")]
    _estimation: Option<serde_json::Number>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum CardCustomFieldValue {
    Choice0(String),
    Choice1(serde_json::Number),
    Choice2(bool),
    Choice3(DiscardVec<Option<String>>),
}

#[derive(Deserialize)]
pub(super) enum CardsType {
    #[serde(rename = "cardType-card")]
    Choice0,
    #[serde(rename = "cardType-linkedCard")]
    Choice1,
    #[serde(rename = "cardType-linkedBoard")]
    Choice2,
    #[serde(rename = "template-card")]
    Choice3,
}

#[derive(Deserialize)]
pub(super) enum ExportRuleButtonType {
    #[serde(rename = "card")]
    Choice0,
    #[serde(rename = "board")]
    Choice1,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum ExportTriggerWeekday {
    Choice0(String),
    Choice1(serde_json::Number),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum ExportTriggerDayOfMonth {
    Choice0(String),
    Choice1(serde_json::Number),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum ExportTriggerDays {
    Choice0(String),
    Choice1(serde_json::Number),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum ExportActionDays {
    Choice0(String),
    Choice1(serde_json::Number),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExportUserProfile {
    #[serde(rename = "fullname", default, deserialize_with = "optional")]
    _fullname: Option<Option<String>>,
    #[serde(rename = "initials", default, deserialize_with = "optional")]
    _initials: Option<Option<String>>,
    #[serde(rename = "avatarUrl", default, deserialize_with = "optional")]
    _avatar_url: Option<Option<String>>,
    #[serde(rename = "avatarFile", default, deserialize_with = "optional")]
    _avatar_file: Option<Base64>,
    #[serde(rename = "avatarFileName", default, deserialize_with = "optional")]
    _avatar_file_name: Option<String>,
    #[serde(rename = "avatarFileType", default, deserialize_with = "optional")]
    _avatar_file_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeckExportStacksItemCardsItem {
    #[serde(rename = "title", deserialize_with = "required_nullable")]
    _title: Option<String>,
    #[serde(rename = "description")]
    _description: String,
    #[serde(rename = "duedate", default, deserialize_with = "optional")]
    _duedate: Option<Date>,
    #[serde(rename = "labels")]
    _labels: DiscardVec<DeckExportStacksItemCardsItemLabelsItem>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OpenprojectExportEmbeddedElementsItem {
    #[serde(rename = "subject", deserialize_with = "required_nullable")]
    _subject: Option<String>,
    #[serde(rename = "description")]
    _description: OpenprojectExportEmbeddedElementsItemDescription,
    #[serde(rename = "dueDate", default, deserialize_with = "optional")]
    _due_date: Option<Date>,
    #[serde(rename = "_links")]
    __links: OpenprojectExportEmbeddedElementsItemLinks,
}

#[derive(Deserialize)]
pub(super) enum TrelloExportPrefsBackground {
    #[serde(rename = "blue")]
    Choice0,
}

#[derive(Deserialize)]
pub(super) enum TrelloExportPrefsPermissionLevel {
    #[serde(rename = "private")]
    Choice0,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JiraExportIssuesItemFields {
    #[serde(rename = "summary", deserialize_with = "required_nullable")]
    _summary: Option<String>,
    #[serde(rename = "description")]
    _description: String,
    #[serde(rename = "status")]
    _status: JiraExportIssuesItemFieldsStatus,
    #[serde(rename = "labels")]
    _labels: DiscardVec<String>,
    #[serde(rename = "duedate", default, deserialize_with = "optional")]
    _duedate: Option<Date>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AsanaExportDataItemMembershipsItem {
    #[serde(rename = "section")]
    _section: AsanaExportDataItemMembershipsItemSection,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AsanaExportDataItemTagsItem {
    #[serde(rename = "name")]
    _name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CustomFieldsSettingsDropdownitems {
    #[serde(rename = "_id")]
    __id: String,
    #[serde(rename = "name")]
    _name: String,
}

#[derive(Deserialize)]
pub(super) enum CardsStickersHighlight {
    #[serde(rename = "underline")]
    Choice0,
    #[serde(rename = "round")]
    Choice1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeckExportStacksItemCardsItemLabelsItem {
    #[serde(rename = "title")]
    _title: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OpenprojectExportEmbeddedElementsItemDescription {
    #[serde(rename = "raw")]
    _raw: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OpenprojectExportEmbeddedElementsItemLinks {
    #[serde(rename = "status")]
    _status: OpenprojectExportEmbeddedElementsItemLinksStatus,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JiraExportIssuesItemFieldsStatus {
    #[serde(rename = "name")]
    _name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AsanaExportDataItemMembershipsItemSection {
    #[serde(rename = "name")]
    _name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OpenprojectExportEmbeddedElementsItemLinksStatus {
    #[serde(rename = "title")]
    _title: String,
}
