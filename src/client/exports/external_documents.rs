//! v12.57 external artifact records. Shapes verified against the matching formatter modules.
#![allow(dead_code)]
use super::validation::Never;
use serde::Deserialize;
use serde_json::Number;
use std::collections::BTreeMap;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaskwarriorExportItemAnnotationsItem {
    #[serde(rename = "entry")]
    pub entry: String,
    #[serde(rename = "description")]
    pub description: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaskwarriorExportItem {
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(
        rename = "entry",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub entry: Option<String>,
    #[serde(
        rename = "annotations",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub annotations: Option<Vec<TaskwarriorExportItemAnnotationsItem>>,
    #[serde(rename = "wekanlist")]
    pub wekanlist: String,
    #[serde(
        rename = "due",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due: Option<String>,
    #[serde(
        rename = "scheduled",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub scheduled: Option<String>,
    #[serde(
        rename = "project",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub project: Option<String>,
    #[serde(
        rename = "priority",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub priority: Option<String>,
    #[serde(
        rename = "tags",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub tags: Option<Vec<String>>,
    #[serde(
        rename = "wekandescription",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub wekandescription: Option<String>,
    #[serde(
        rename = "end",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub end: Option<String>,
}
pub(super) type TaskwarriorExport = Vec<TaskwarriorExportItem>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NullboardExportListsItemNotesItem {
    #[serde(rename = "text")]
    pub text: String,
    #[serde(rename = "raw")]
    pub raw: bool,
    #[serde(rename = "min")]
    pub min: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NullboardExportListsItem {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "notes")]
    pub notes: Vec<NullboardExportListsItemNotesItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NullboardExport {
    #[serde(rename = "format")]
    pub format: Number,
    #[serde(rename = "id")]
    pub id: Number,
    #[serde(rename = "revision")]
    pub revision: Number,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "lists")]
    pub lists: Vec<NullboardExportListsItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KanriExportColumnsItemCardsItemTasksItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "finished")]
    pub finished: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KanriExportColumnsItemCardsItemTagsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "text")]
    pub text: String,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(rename = "style")]
    pub style: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KanriExportColumnsItemCardsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(
        rename = "dueDate",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub due_date: Option<String>,
    #[serde(rename = "isDueDateCounterRelative")]
    pub is_due_date_counter_relative: bool,
    #[serde(rename = "isDueDateCompleted")]
    pub is_due_date_completed: bool,
    #[serde(rename = "tasks")]
    pub tasks: Vec<KanriExportColumnsItemCardsItemTasksItem>,
    #[serde(rename = "tags")]
    pub tags: Vec<KanriExportColumnsItemCardsItemTagsItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KanriExportColumnsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "cards")]
    pub cards: Vec<KanriExportColumnsItemCardsItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KanriExportGlobalTagsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "text")]
    pub text: String,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(rename = "style")]
    pub style: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct KanriExport {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "columns")]
    pub columns: Vec<KanriExportColumnsItem>,
    #[serde(rename = "background")]
    pub background: (),
    #[serde(rename = "globalTags")]
    pub global_tags: Vec<KanriExportGlobalTagsItem>,
    #[serde(rename = "lastEdited")]
    pub last_edited: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataTasksItemTask {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "creationDate")]
    pub creation_date: Number,
    #[serde(rename = "modificationDate")]
    pub modification_date: Number,
    #[serde(rename = "remoteId")]
    pub remote_id: String,
    #[serde(
        rename = "priority",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub priority: Option<Number>,
    #[serde(
        rename = "dueDate",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due_date: Option<Number>,
    #[serde(
        rename = "hideUntil",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub hide_until: Option<Number>,
    #[serde(
        rename = "notes",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub notes: Option<String>,
    #[serde(
        rename = "estimatedSeconds",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub estimated_seconds: Option<Number>,
    #[serde(
        rename = "elapsedSeconds",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub elapsed_seconds: Option<Number>,
    #[serde(
        rename = "completionDate",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub completion_date: Option<Number>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataTasksItemCommentsItem {
    #[serde(rename = "remoteId")]
    pub remote_id: String,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "created")]
    pub created: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataTasksItemCaldavTasksItem {
    #[serde(rename = "calendar")]
    pub calendar: String,
    #[serde(rename = "remoteId")]
    pub remote_id: String,
    #[serde(rename = "object")]
    pub object: String,
    #[serde(
        rename = "remoteParent",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub remote_parent: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataTasksItemTagsItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "tagUid")]
    pub tag_uid: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataTasksItem {
    #[serde(rename = "task")]
    pub task: TasksorgExportDataTasksItemTask,
    #[serde(
        rename = "comments",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub comments: Option<Vec<TasksorgExportDataTasksItemCommentsItem>>,
    #[serde(rename = "caldavTasks")]
    pub caldav_tasks: Vec<TasksorgExportDataTasksItemCaldavTasksItem>,
    #[serde(
        rename = "tags",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub tags: Option<Vec<TasksorgExportDataTasksItemTagsItem>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataTagsItem {
    #[serde(rename = "remoteId")]
    pub remote_id: String,
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataCaldavAccountsItem {
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "accountType")]
    pub account_type: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataCaldavCalendarsItem {
    #[serde(rename = "account")]
    pub account: String,
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "order")]
    pub order: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataIntPrefs {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataLongPrefs {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataStringPrefs {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataBoolPrefs {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportDataSetPrefs {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExportData {
    #[serde(rename = "tasks")]
    pub tasks: Vec<TasksorgExportDataTasksItem>,
    #[serde(rename = "places")]
    pub places: Vec<Never>,
    #[serde(rename = "tags")]
    pub tags: Vec<TasksorgExportDataTagsItem>,
    #[serde(rename = "filters")]
    pub filters: Vec<Never>,
    #[serde(rename = "caldavAccounts")]
    pub caldav_accounts: Vec<TasksorgExportDataCaldavAccountsItem>,
    #[serde(rename = "caldavCalendars")]
    pub caldav_calendars: Vec<TasksorgExportDataCaldavCalendarsItem>,
    #[serde(rename = "taskListMetadata")]
    pub task_list_metadata: Vec<Never>,
    #[serde(rename = "taskAttachments")]
    pub task_attachments: Vec<Never>,
    #[serde(rename = "intPrefs")]
    pub int_prefs: TasksorgExportDataIntPrefs,
    #[serde(rename = "longPrefs")]
    pub long_prefs: TasksorgExportDataLongPrefs,
    #[serde(rename = "stringPrefs")]
    pub string_prefs: TasksorgExportDataStringPrefs,
    #[serde(rename = "boolPrefs")]
    pub bool_prefs: TasksorgExportDataBoolPrefs,
    #[serde(rename = "setPrefs")]
    pub set_prefs: TasksorgExportDataSetPrefs,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TasksorgExport {
    #[serde(rename = "version")]
    pub version: Number,
    #[serde(rename = "timestamp")]
    pub timestamp: Number,
    #[serde(rename = "data")]
    pub data: TasksorgExportData,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataProjectEntitiesEntityTheme {
    #[serde(rename = "isAutoContrast")]
    pub is_auto_contrast: bool,
    #[serde(rename = "isDisableBackgroundTint")]
    pub is_disable_background_tint: bool,
    #[serde(rename = "primary")]
    pub primary: String,
    #[serde(rename = "huePrimary")]
    pub hue_primary: String,
    #[serde(rename = "accent")]
    pub accent: String,
    #[serde(rename = "hueAccent")]
    pub hue_accent: String,
    #[serde(rename = "warn")]
    pub warn: String,
    #[serde(rename = "hueWarn")]
    pub hue_warn: String,
    #[serde(rename = "backgroundImageDark")]
    pub background_image_dark: (),
    #[serde(rename = "backgroundImageLight")]
    pub background_image_light: (),
    #[serde(rename = "backgroundOverlayOpacity")]
    pub background_overlay_opacity: Number,
    #[serde(rename = "backgroundImageBlur")]
    pub background_image_blur: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataProjectEntitiesEntityAdvancedCfgWorklogExportSettings {
    #[serde(rename = "cols")]
    pub cols: Vec<String>,
    #[serde(rename = "roundWorkTimeTo")]
    pub round_work_time_to: (),
    #[serde(rename = "roundStartTimeTo")]
    pub round_start_time_to: (),
    #[serde(rename = "roundEndTimeTo")]
    pub round_end_time_to: (),
    #[serde(rename = "separateTasksBy")]
    pub separate_tasks_by: String,
    #[serde(rename = "groupBy")]
    pub group_by: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataProjectEntitiesEntityAdvancedCfg {
    #[serde(rename = "worklogExportSettings")]
    pub worklog_export_settings:
        SuperproductivityExportDataProjectEntitiesEntityAdvancedCfgWorklogExportSettings,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataProjectEntitiesEntity {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "isHiddenFromMenu")]
    pub is_hidden_from_menu: bool,
    #[serde(rename = "isArchived")]
    pub is_archived: bool,
    #[serde(rename = "isEnableBacklog")]
    pub is_enable_backlog: bool,
    #[serde(rename = "taskIds")]
    pub task_ids: Vec<String>,
    #[serde(rename = "backlogTaskIds")]
    pub backlog_task_ids: Vec<String>,
    #[serde(rename = "noteIds")]
    pub note_ids: Vec<String>,
    #[serde(rename = "icon")]
    pub icon: (),
    #[serde(rename = "theme")]
    pub theme: SuperproductivityExportDataProjectEntitiesEntityTheme,
    #[serde(rename = "advancedCfg")]
    pub advanced_cfg: SuperproductivityExportDataProjectEntitiesEntityAdvancedCfg,
    #[serde(rename = "created")]
    pub created: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataProject {
    #[serde(rename = "ids")]
    pub ids: Vec<String>,
    #[serde(rename = "entities")]
    pub entities: BTreeMap<String, SuperproductivityExportDataProjectEntitiesEntity>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataTaskEntitiesEntity {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "subTaskIds")]
    pub sub_task_ids: Vec<String>,
    #[serde(rename = "timeSpentOnDay")]
    pub time_spent_on_day: BTreeMap<String, Number>,
    #[serde(rename = "timeSpent")]
    pub time_spent: Number,
    #[serde(rename = "timeEstimate")]
    pub time_estimate: Number,
    #[serde(rename = "isDone")]
    pub is_done: bool,
    #[serde(rename = "notes")]
    pub notes: String,
    #[serde(rename = "tagIds")]
    pub tag_ids: Vec<String>,
    #[serde(rename = "created")]
    pub created: Number,
    #[serde(rename = "attachments")]
    pub attachments: Vec<Never>,
    #[serde(rename = "projectId")]
    pub project_id: String,
    #[serde(
        rename = "dueWithTime",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due_with_time: Option<Number>,
    #[serde(
        rename = "priority",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub priority: Option<Number>,
    #[serde(
        rename = "parentId",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub parent_id: Option<String>,
    #[serde(
        rename = "doneOn",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub done_on: Option<Number>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataTask {
    #[serde(rename = "ids")]
    pub ids: Vec<String>,
    #[serde(rename = "entities")]
    pub entities: BTreeMap<String, SuperproductivityExportDataTaskEntitiesEntity>,
    #[serde(rename = "currentTaskId")]
    pub current_task_id: (),
    #[serde(rename = "selectedTaskId")]
    pub selected_task_id: (),
    #[serde(rename = "lastCurrentTaskId")]
    pub last_current_task_id: (),
    #[serde(rename = "isDataLoaded")]
    pub is_data_loaded: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataTagEntitiesEntityTheme {
    #[serde(rename = "isAutoContrast")]
    pub is_auto_contrast: bool,
    #[serde(rename = "isDisableBackgroundTint")]
    pub is_disable_background_tint: bool,
    #[serde(rename = "primary")]
    pub primary: String,
    #[serde(rename = "huePrimary")]
    pub hue_primary: String,
    #[serde(rename = "accent")]
    pub accent: String,
    #[serde(rename = "hueAccent")]
    pub hue_accent: String,
    #[serde(rename = "warn")]
    pub warn: String,
    #[serde(rename = "hueWarn")]
    pub hue_warn: String,
    #[serde(rename = "backgroundImageDark")]
    pub background_image_dark: (),
    #[serde(rename = "backgroundImageLight")]
    pub background_image_light: (),
    #[serde(rename = "backgroundOverlayOpacity")]
    pub background_overlay_opacity: Number,
    #[serde(rename = "backgroundImageBlur")]
    pub background_image_blur: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataTagEntitiesEntityAdvancedCfgWorklogExportSettings {
    #[serde(rename = "cols")]
    pub cols: Vec<String>,
    #[serde(rename = "roundWorkTimeTo")]
    pub round_work_time_to: (),
    #[serde(rename = "roundStartTimeTo")]
    pub round_start_time_to: (),
    #[serde(rename = "roundEndTimeTo")]
    pub round_end_time_to: (),
    #[serde(rename = "separateTasksBy")]
    pub separate_tasks_by: String,
    #[serde(rename = "groupBy")]
    pub group_by: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataTagEntitiesEntityAdvancedCfg {
    #[serde(rename = "worklogExportSettings")]
    pub worklog_export_settings:
        SuperproductivityExportDataTagEntitiesEntityAdvancedCfgWorklogExportSettings,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataTagEntitiesEntity {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "color")]
    pub color: (),
    #[serde(rename = "created")]
    pub created: Number,
    #[serde(rename = "taskIds")]
    pub task_ids: Vec<String>,
    #[serde(
        rename = "icon",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub icon: Option<String>,
    #[serde(rename = "theme")]
    pub theme: SuperproductivityExportDataTagEntitiesEntityTheme,
    #[serde(rename = "advancedCfg")]
    pub advanced_cfg: SuperproductivityExportDataTagEntitiesEntityAdvancedCfg,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataTag {
    #[serde(rename = "ids")]
    pub ids: Vec<String>,
    #[serde(rename = "entities")]
    pub entities: BTreeMap<String, SuperproductivityExportDataTagEntitiesEntity>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataNote {
    #[serde(rename = "ids")]
    pub ids: Vec<String>,
    #[serde(rename = "entities")]
    pub entities: BTreeMap<String, Never>,
    #[serde(rename = "todayOrder")]
    pub today_order: Vec<Never>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataBoardsBoardCfgsItemPanelsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "taskIds")]
    pub task_ids: Vec<String>,
    #[serde(rename = "includedTagIds")]
    pub included_tag_ids: Vec<String>,
    #[serde(rename = "excludedTagIds")]
    pub excluded_tag_ids: Vec<String>,
    #[serde(rename = "projectIds")]
    pub project_ids: Vec<String>,
    #[serde(rename = "taskDoneState")]
    pub task_done_state: Number,
    #[serde(rename = "scheduledState")]
    pub scheduled_state: Number,
    #[serde(rename = "backlogState")]
    pub backlog_state: Number,
    #[serde(rename = "isParentTasksOnly")]
    pub is_parent_tasks_only: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataBoardsBoardCfgsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "cols")]
    pub cols: Number,
    #[serde(rename = "panels")]
    pub panels: Vec<SuperproductivityExportDataBoardsBoardCfgsItemPanelsItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataBoards {
    #[serde(rename = "boardCfgs")]
    pub board_cfgs: Vec<SuperproductivityExportDataBoardsBoardCfgsItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataArchiveYoungTask {
    #[serde(rename = "ids")]
    pub ids: Vec<String>,
    #[serde(rename = "entities")]
    pub entities: BTreeMap<String, Never>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataArchiveYoungTimeTrackingProject {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataArchiveYoungTimeTrackingTag {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataArchiveYoungTimeTracking {
    #[serde(rename = "project")]
    pub project: SuperproductivityExportDataArchiveYoungTimeTrackingProject,
    #[serde(rename = "tag")]
    pub tag: SuperproductivityExportDataArchiveYoungTimeTrackingTag,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataArchiveYoung {
    #[serde(rename = "task")]
    pub task: SuperproductivityExportDataArchiveYoungTask,
    #[serde(rename = "timeTracking")]
    pub time_tracking: SuperproductivityExportDataArchiveYoungTimeTracking,
    #[serde(rename = "lastTimeTrackingFlush")]
    pub last_time_tracking_flush: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataArchiveOldTask {
    #[serde(rename = "ids")]
    pub ids: Vec<String>,
    #[serde(rename = "entities")]
    pub entities: BTreeMap<String, Never>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataArchiveOldTimeTrackingProject {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataArchiveOldTimeTrackingTag {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataArchiveOldTimeTracking {
    #[serde(rename = "project")]
    pub project: SuperproductivityExportDataArchiveOldTimeTrackingProject,
    #[serde(rename = "tag")]
    pub tag: SuperproductivityExportDataArchiveOldTimeTrackingTag,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportDataArchiveOld {
    #[serde(rename = "task")]
    pub task: SuperproductivityExportDataArchiveOldTask,
    #[serde(rename = "timeTracking")]
    pub time_tracking: SuperproductivityExportDataArchiveOldTimeTracking,
    #[serde(rename = "lastTimeTrackingFlush")]
    pub last_time_tracking_flush: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExportData {
    #[serde(rename = "project")]
    pub project: SuperproductivityExportDataProject,
    #[serde(rename = "task")]
    pub task: SuperproductivityExportDataTask,
    #[serde(rename = "tag")]
    pub tag: SuperproductivityExportDataTag,
    #[serde(rename = "note")]
    pub note: SuperproductivityExportDataNote,
    #[serde(rename = "boards")]
    pub boards: SuperproductivityExportDataBoards,
    #[serde(rename = "archiveYoung")]
    pub archive_young: SuperproductivityExportDataArchiveYoung,
    #[serde(rename = "archiveOld")]
    pub archive_old: SuperproductivityExportDataArchiveOld,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SuperproductivityExport {
    #[serde(rename = "timestamp")]
    pub timestamp: Number,
    #[serde(rename = "lastUpdate")]
    pub last_update: Number,
    #[serde(rename = "crossModelVersion")]
    pub cross_model_version: Number,
    #[serde(rename = "data")]
    pub data: SuperproductivityExportData,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportRolesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "slug")]
    pub slug: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "computable")]
    pub computable: bool,
    #[serde(rename = "permissions")]
    pub permissions: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportPointsItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(
        rename = "value",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub value: Option<Number>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportEpicStatusesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "slug")]
    pub slug: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "is_closed")]
    pub is_closed: bool,
    #[serde(rename = "color")]
    pub color: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportUsStatusesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "slug")]
    pub slug: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "is_closed")]
    pub is_closed: bool,
    #[serde(rename = "is_archived")]
    pub is_archived: bool,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(rename = "wip_limit")]
    pub wip_limit: (),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportUsDuedatesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "by_default")]
    pub by_default: bool,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(
        rename = "days_to_due",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub days_to_due: Option<Number>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportTaskStatusesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "slug")]
    pub slug: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "is_closed")]
    pub is_closed: bool,
    #[serde(rename = "color")]
    pub color: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportTaskDuedatesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "by_default")]
    pub by_default: bool,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(
        rename = "days_to_due",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub days_to_due: Option<Number>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportIssueTypesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "color")]
    pub color: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportIssueStatusesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "slug")]
    pub slug: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "is_closed")]
    pub is_closed: bool,
    #[serde(rename = "color")]
    pub color: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportIssueDuedatesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "by_default")]
    pub by_default: bool,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(
        rename = "days_to_due",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub days_to_due: Option<Number>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportPrioritiesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "color")]
    pub color: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportSeveritiesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "color")]
    pub color: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportSwimlanesItemStatusesItem {
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "wip_limit")]
    pub wip_limit: (),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportSwimlanesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "statuses")]
    pub statuses: Vec<TaigaExportSwimlanesItemStatusesItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportEpiccustomattributesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "created_date")]
    pub created_date: String,
    #[serde(rename = "modified_date")]
    pub modified_date: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportUserstorycustomattributesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "created_date")]
    pub created_date: String,
    #[serde(rename = "modified_date")]
    pub modified_date: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportTaskcustomattributesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "created_date")]
    pub created_date: String,
    #[serde(rename = "modified_date")]
    pub modified_date: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportIssuecustomattributesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "created_date")]
    pub created_date: String,
    #[serde(rename = "modified_date")]
    pub modified_date: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportEpicsItemHistoryItemDiff {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportEpicsItemHistoryItemValues {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportEpicsItemHistoryItem {
    #[serde(rename = "user")]
    pub user: Vec<String>,
    #[serde(rename = "diff")]
    pub diff: TaigaExportEpicsItemHistoryItemDiff,
    #[serde(rename = "snapshot")]
    pub snapshot: (),
    #[serde(rename = "values")]
    pub values: TaigaExportEpicsItemHistoryItemValues,
    #[serde(rename = "comment")]
    pub comment: String,
    #[serde(rename = "delete_comment_date")]
    pub delete_comment_date: (),
    #[serde(rename = "delete_comment_user")]
    pub delete_comment_user: Vec<Never>,
    #[serde(rename = "comment_versions")]
    pub comment_versions: (),
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "edit_comment_date")]
    pub edit_comment_date: (),
    #[serde(rename = "is_hidden")]
    pub is_hidden: bool,
    #[serde(rename = "is_snapshot")]
    pub is_snapshot: bool,
    #[serde(rename = "type")]
    pub r#type: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportEpicsItem {
    #[serde(rename = "owner")]
    pub owner: String,
    #[serde(rename = "assigned_to")]
    pub assigned_to: String,
    #[serde(rename = "modified_date")]
    pub modified_date: String,
    #[serde(rename = "created_date")]
    pub created_date: String,
    #[serde(rename = "ref")]
    pub r#ref: Number,
    #[serde(rename = "subject")]
    pub subject: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "version")]
    pub version: Number,
    #[serde(rename = "blocked_note")]
    pub blocked_note: String,
    #[serde(rename = "is_blocked")]
    pub is_blocked: bool,
    #[serde(rename = "attachments")]
    pub attachments: Vec<Never>,
    #[serde(rename = "history")]
    pub history: Vec<TaigaExportEpicsItemHistoryItem>,
    #[serde(rename = "watchers")]
    pub watchers: Vec<Never>,
    #[serde(rename = "custom_attributes_values")]
    pub custom_attributes_values: BTreeMap<String, AttributeValue>,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "epics_order")]
    pub epics_order: Number,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(rename = "client_requirement")]
    pub client_requirement: bool,
    #[serde(rename = "team_requirement")]
    pub team_requirement: bool,
    #[serde(rename = "tags")]
    pub tags: Vec<String>,
    #[serde(rename = "related_user_stories")]
    pub related_user_stories: Vec<Never>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportUserStoriesItemRolePointsItem {
    #[serde(rename = "role")]
    pub role: String,
    #[serde(rename = "points")]
    pub points: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportUserStoriesItemHistoryItemDiff {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportUserStoriesItemHistoryItemValues {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportUserStoriesItemHistoryItem {
    #[serde(rename = "user")]
    pub user: Vec<String>,
    #[serde(rename = "diff")]
    pub diff: TaigaExportUserStoriesItemHistoryItemDiff,
    #[serde(rename = "snapshot")]
    pub snapshot: (),
    #[serde(rename = "values")]
    pub values: TaigaExportUserStoriesItemHistoryItemValues,
    #[serde(rename = "comment")]
    pub comment: String,
    #[serde(rename = "delete_comment_date")]
    pub delete_comment_date: (),
    #[serde(rename = "delete_comment_user")]
    pub delete_comment_user: Vec<Never>,
    #[serde(rename = "comment_versions")]
    pub comment_versions: (),
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "edit_comment_date")]
    pub edit_comment_date: (),
    #[serde(rename = "is_hidden")]
    pub is_hidden: bool,
    #[serde(rename = "is_snapshot")]
    pub is_snapshot: bool,
    #[serde(rename = "type")]
    pub r#type: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportUserStoriesItem {
    #[serde(rename = "watchers")]
    pub watchers: Vec<Never>,
    #[serde(rename = "role_points")]
    pub role_points: Vec<TaigaExportUserStoriesItemRolePointsItem>,
    #[serde(
        rename = "owner",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub owner: Option<String>,
    #[serde(
        rename = "assigned_to",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub assigned_to: Option<String>,
    #[serde(rename = "assigned_users")]
    pub assigned_users: Vec<String>,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(
        rename = "swimlane",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub swimlane: Option<String>,
    #[serde(
        rename = "milestone",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub milestone: Option<String>,
    #[serde(rename = "modified_date")]
    pub modified_date: String,
    #[serde(rename = "created_date")]
    pub created_date: String,
    #[serde(
        rename = "finish_date",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub finish_date: Option<String>,
    #[serde(rename = "generated_from_issue")]
    pub generated_from_issue: (),
    #[serde(rename = "generated_from_task")]
    pub generated_from_task: (),
    #[serde(rename = "from_task_ref")]
    pub from_task_ref: (),
    #[serde(rename = "ref")]
    pub r#ref: Number,
    #[serde(rename = "is_closed")]
    pub is_closed: bool,
    #[serde(rename = "backlog_order")]
    pub backlog_order: Number,
    #[serde(rename = "sprint_order")]
    pub sprint_order: Number,
    #[serde(rename = "kanban_order")]
    pub kanban_order: Number,
    #[serde(rename = "subject")]
    pub subject: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "client_requirement")]
    pub client_requirement: bool,
    #[serde(rename = "team_requirement")]
    pub team_requirement: bool,
    #[serde(rename = "external_reference")]
    pub external_reference: (),
    #[serde(rename = "tribe_gig")]
    pub tribe_gig: (),
    #[serde(rename = "version")]
    pub version: Number,
    #[serde(rename = "blocked_note")]
    pub blocked_note: String,
    #[serde(rename = "is_blocked")]
    pub is_blocked: bool,
    #[serde(rename = "tags")]
    pub tags: Vec<String>,
    #[serde(
        rename = "due_date",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub due_date: Option<String>,
    #[serde(rename = "due_date_reason")]
    pub due_date_reason: String,
    #[serde(rename = "attachments")]
    pub attachments: Vec<Never>,
    #[serde(rename = "history")]
    pub history: Vec<TaigaExportUserStoriesItemHistoryItem>,
    #[serde(rename = "custom_attributes_values")]
    pub custom_attributes_values: BTreeMap<String, AttributeValue>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportTasksItemHistoryItemDiff {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportTasksItemHistoryItemValues {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportTasksItemHistoryItem {
    #[serde(rename = "user")]
    pub user: Vec<String>,
    #[serde(rename = "diff")]
    pub diff: TaigaExportTasksItemHistoryItemDiff,
    #[serde(rename = "snapshot")]
    pub snapshot: (),
    #[serde(rename = "values")]
    pub values: TaigaExportTasksItemHistoryItemValues,
    #[serde(rename = "comment")]
    pub comment: String,
    #[serde(rename = "delete_comment_date")]
    pub delete_comment_date: (),
    #[serde(rename = "delete_comment_user")]
    pub delete_comment_user: Vec<Never>,
    #[serde(rename = "comment_versions")]
    pub comment_versions: (),
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "edit_comment_date")]
    pub edit_comment_date: (),
    #[serde(rename = "is_hidden")]
    pub is_hidden: bool,
    #[serde(rename = "is_snapshot")]
    pub is_snapshot: bool,
    #[serde(rename = "type")]
    pub r#type: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportTasksItem {
    #[serde(
        rename = "owner",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub owner: Option<String>,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "user_story")]
    pub user_story: Number,
    #[serde(
        rename = "milestone",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub milestone: Option<String>,
    #[serde(
        rename = "assigned_to",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub assigned_to: Option<String>,
    #[serde(rename = "modified_date")]
    pub modified_date: String,
    #[serde(rename = "created_date")]
    pub created_date: String,
    #[serde(
        rename = "finished_date",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub finished_date: Option<String>,
    #[serde(rename = "ref")]
    pub r#ref: Number,
    #[serde(rename = "subject")]
    pub subject: String,
    #[serde(rename = "us_order")]
    pub us_order: Number,
    #[serde(rename = "taskboard_order")]
    pub taskboard_order: Number,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "is_iocaine")]
    pub is_iocaine: bool,
    #[serde(rename = "external_reference")]
    pub external_reference: (),
    #[serde(rename = "version")]
    pub version: Number,
    #[serde(rename = "blocked_note")]
    pub blocked_note: String,
    #[serde(rename = "is_blocked")]
    pub is_blocked: bool,
    #[serde(rename = "tags")]
    pub tags: Vec<String>,
    #[serde(
        rename = "due_date",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub due_date: Option<String>,
    #[serde(rename = "due_date_reason")]
    pub due_date_reason: String,
    #[serde(rename = "attachments")]
    pub attachments: Vec<Never>,
    #[serde(rename = "history")]
    pub history: Vec<TaigaExportTasksItemHistoryItem>,
    #[serde(rename = "watchers")]
    pub watchers: Vec<Never>,
    #[serde(rename = "custom_attributes_values")]
    pub custom_attributes_values: BTreeMap<String, AttributeValue>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportMilestonesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "owner")]
    pub owner: (),
    #[serde(rename = "created_date")]
    pub created_date: String,
    #[serde(rename = "modified_date")]
    pub modified_date: String,
    #[serde(rename = "estimated_start")]
    pub estimated_start: String,
    #[serde(rename = "estimated_finish")]
    pub estimated_finish: String,
    #[serde(rename = "slug")]
    pub slug: String,
    #[serde(rename = "closed")]
    pub closed: bool,
    #[serde(rename = "disponibility")]
    pub disponibility: Number,
    #[serde(rename = "order")]
    pub order: Number,
    #[serde(rename = "watchers")]
    pub watchers: Vec<Never>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportIssuesItemHistoryItemDiff {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportIssuesItemHistoryItemValues {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportIssuesItemHistoryItem {
    #[serde(rename = "user")]
    pub user: Vec<String>,
    #[serde(rename = "diff")]
    pub diff: TaigaExportIssuesItemHistoryItemDiff,
    #[serde(rename = "snapshot")]
    pub snapshot: (),
    #[serde(rename = "values")]
    pub values: TaigaExportIssuesItemHistoryItemValues,
    #[serde(rename = "comment")]
    pub comment: String,
    #[serde(rename = "delete_comment_date")]
    pub delete_comment_date: (),
    #[serde(rename = "delete_comment_user")]
    pub delete_comment_user: Vec<Never>,
    #[serde(rename = "comment_versions")]
    pub comment_versions: (),
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "edit_comment_date")]
    pub edit_comment_date: (),
    #[serde(rename = "is_hidden")]
    pub is_hidden: bool,
    #[serde(rename = "is_snapshot")]
    pub is_snapshot: bool,
    #[serde(rename = "type")]
    pub r#type: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExportIssuesItem {
    #[serde(rename = "owner")]
    pub owner: String,
    #[serde(rename = "assigned_to")]
    pub assigned_to: String,
    #[serde(rename = "modified_date")]
    pub modified_date: String,
    #[serde(rename = "created_date")]
    pub created_date: String,
    #[serde(rename = "ref")]
    pub r#ref: Number,
    #[serde(rename = "subject")]
    pub subject: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "version")]
    pub version: Number,
    #[serde(rename = "blocked_note")]
    pub blocked_note: String,
    #[serde(rename = "is_blocked")]
    pub is_blocked: bool,
    #[serde(rename = "attachments")]
    pub attachments: Vec<Never>,
    #[serde(rename = "history")]
    pub history: Vec<TaigaExportIssuesItemHistoryItem>,
    #[serde(rename = "watchers")]
    pub watchers: Vec<Never>,
    #[serde(rename = "custom_attributes_values")]
    pub custom_attributes_values: BTreeMap<String, AttributeValue>,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "priority")]
    pub priority: String,
    #[serde(rename = "severity")]
    pub severity: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "milestone")]
    pub milestone: (),
    #[serde(rename = "votes")]
    pub votes: Vec<Never>,
    #[serde(rename = "finished_date")]
    pub finished_date: (),
    #[serde(rename = "external_reference")]
    pub external_reference: (),
    #[serde(rename = "tags")]
    pub tags: Vec<String>,
    #[serde(rename = "due_date")]
    pub due_date: String,
    #[serde(rename = "due_date_reason")]
    pub due_date_reason: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TaigaExport {
    #[serde(rename = "watchers")]
    pub watchers: Vec<Never>,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "slug")]
    pub slug: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "created_date")]
    pub created_date: String,
    #[serde(rename = "logo")]
    pub logo: (),
    #[serde(
        rename = "total_milestones",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub total_milestones: Option<Number>,
    #[serde(rename = "total_story_points")]
    pub total_story_points: (),
    #[serde(rename = "is_epics_activated")]
    pub is_epics_activated: bool,
    #[serde(rename = "is_backlog_activated")]
    pub is_backlog_activated: bool,
    #[serde(rename = "is_kanban_activated")]
    pub is_kanban_activated: bool,
    #[serde(rename = "is_wiki_activated")]
    pub is_wiki_activated: bool,
    #[serde(rename = "is_issues_activated")]
    pub is_issues_activated: bool,
    #[serde(rename = "videoconferences")]
    pub videoconferences: (),
    #[serde(rename = "videoconferences_extra_data")]
    pub videoconferences_extra_data: (),
    #[serde(rename = "creation_template")]
    pub creation_template: (),
    #[serde(rename = "is_private")]
    pub is_private: bool,
    #[serde(rename = "is_featured")]
    pub is_featured: bool,
    #[serde(rename = "is_looking_for_people")]
    pub is_looking_for_people: bool,
    #[serde(rename = "looking_for_people_note")]
    pub looking_for_people_note: String,
    #[serde(rename = "epics_csv_uuid")]
    pub epics_csv_uuid: (),
    #[serde(rename = "userstories_csv_uuid")]
    pub userstories_csv_uuid: (),
    #[serde(rename = "tasks_csv_uuid")]
    pub tasks_csv_uuid: (),
    #[serde(rename = "issues_csv_uuid")]
    pub issues_csv_uuid: (),
    #[serde(rename = "transfer_token")]
    pub transfer_token: (),
    #[serde(rename = "blocked_code")]
    pub blocked_code: (),
    #[serde(rename = "totals_updated_datetime")]
    pub totals_updated_datetime: String,
    #[serde(rename = "total_fans")]
    pub total_fans: Number,
    #[serde(rename = "total_fans_last_week")]
    pub total_fans_last_week: Number,
    #[serde(rename = "total_fans_last_month")]
    pub total_fans_last_month: Number,
    #[serde(rename = "total_fans_last_year")]
    pub total_fans_last_year: Number,
    #[serde(rename = "total_activity")]
    pub total_activity: Number,
    #[serde(rename = "total_activity_last_week")]
    pub total_activity_last_week: Number,
    #[serde(rename = "total_activity_last_month")]
    pub total_activity_last_month: Number,
    #[serde(rename = "total_activity_last_year")]
    pub total_activity_last_year: Number,
    #[serde(rename = "anon_permissions")]
    pub anon_permissions: Vec<Never>,
    #[serde(rename = "public_permissions")]
    pub public_permissions: Vec<Never>,
    #[serde(rename = "modified_date")]
    pub modified_date: String,
    #[serde(rename = "roles")]
    pub roles: Vec<TaigaExportRolesItem>,
    #[serde(rename = "owner")]
    pub owner: (),
    #[serde(rename = "memberships")]
    pub memberships: Vec<Never>,
    #[serde(rename = "points")]
    pub points: Vec<TaigaExportPointsItem>,
    #[serde(rename = "epic_statuses")]
    pub epic_statuses: Vec<TaigaExportEpicStatusesItem>,
    #[serde(rename = "us_statuses")]
    pub us_statuses: Vec<TaigaExportUsStatusesItem>,
    #[serde(rename = "us_duedates")]
    pub us_duedates: Vec<TaigaExportUsDuedatesItem>,
    #[serde(rename = "task_statuses")]
    pub task_statuses: Vec<TaigaExportTaskStatusesItem>,
    #[serde(rename = "task_duedates")]
    pub task_duedates: Vec<TaigaExportTaskDuedatesItem>,
    #[serde(rename = "issue_types")]
    pub issue_types: Vec<TaigaExportIssueTypesItem>,
    #[serde(rename = "issue_statuses")]
    pub issue_statuses: Vec<TaigaExportIssueStatusesItem>,
    #[serde(rename = "issue_duedates")]
    pub issue_duedates: Vec<TaigaExportIssueDuedatesItem>,
    #[serde(rename = "priorities")]
    pub priorities: Vec<TaigaExportPrioritiesItem>,
    #[serde(rename = "severities")]
    pub severities: Vec<TaigaExportSeveritiesItem>,
    #[serde(rename = "swimlanes")]
    pub swimlanes: Vec<TaigaExportSwimlanesItem>,
    #[serde(rename = "tags_colors")]
    pub tags_colors: Vec<Vec<String>>,
    #[serde(rename = "default_points")]
    pub default_points: String,
    #[serde(rename = "default_epic_status")]
    pub default_epic_status: String,
    #[serde(rename = "default_us_status")]
    pub default_us_status: String,
    #[serde(rename = "default_task_status")]
    pub default_task_status: String,
    #[serde(rename = "default_priority")]
    pub default_priority: String,
    #[serde(rename = "default_severity")]
    pub default_severity: String,
    #[serde(rename = "default_issue_status")]
    pub default_issue_status: String,
    #[serde(rename = "default_issue_type")]
    pub default_issue_type: String,
    #[serde(
        rename = "default_swimlane",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub default_swimlane: Option<String>,
    #[serde(rename = "epiccustomattributes")]
    pub epiccustomattributes: Vec<TaigaExportEpiccustomattributesItem>,
    #[serde(rename = "userstorycustomattributes")]
    pub userstorycustomattributes: Vec<TaigaExportUserstorycustomattributesItem>,
    #[serde(rename = "taskcustomattributes")]
    pub taskcustomattributes: Vec<TaigaExportTaskcustomattributesItem>,
    #[serde(rename = "issuecustomattributes")]
    pub issuecustomattributes: Vec<TaigaExportIssuecustomattributesItem>,
    #[serde(rename = "epics")]
    pub epics: Vec<TaigaExportEpicsItem>,
    #[serde(rename = "user_stories")]
    pub user_stories: Vec<TaigaExportUserStoriesItem>,
    #[serde(rename = "tasks")]
    pub tasks: Vec<TaigaExportTasksItem>,
    #[serde(rename = "milestones")]
    pub milestones: Vec<TaigaExportMilestonesItem>,
    #[serde(rename = "issues")]
    pub issues: Vec<TaigaExportIssuesItem>,
    #[serde(rename = "wiki_links")]
    pub wiki_links: Vec<Never>,
    #[serde(rename = "wiki_pages")]
    pub wiki_pages: Vec<Never>,
    #[serde(rename = "tags")]
    pub tags: Vec<String>,
    #[serde(rename = "timeline")]
    pub timeline: Vec<Never>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WrikeworkflowExportDataItemCustomStatusesItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "standardName")]
    pub standard_name: bool,
    #[serde(rename = "color")]
    pub color: String,
    #[serde(rename = "standard")]
    pub standard: bool,
    #[serde(rename = "group")]
    pub group: String,
    #[serde(rename = "hidden")]
    pub hidden: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WrikeworkflowExportDataItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "standard")]
    pub standard: bool,
    #[serde(rename = "hidden")]
    pub hidden: bool,
    #[serde(rename = "customStatuses")]
    pub custom_statuses: Vec<WrikeworkflowExportDataItemCustomStatusesItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WrikeworkflowExport {
    #[serde(rename = "kind")]
    pub kind: String,
    #[serde(rename = "data")]
    pub data: Vec<WrikeworkflowExportDataItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeDeckExportStacksItemCardsItemLabelsItem {
    #[serde(rename = "title")]
    pub title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeDeckExportStacksItemCardsItemOwner {
    #[serde(rename = "uid")]
    pub uid: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeDeckExportStacksItemCardsItemCommentsItem {
    #[serde(rename = "message")]
    pub message: String,
    #[serde(
        rename = "actorId",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub actor_id: Option<String>,
    #[serde(rename = "creationDateTime")]
    pub creation_date_time: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeDeckExportStacksItemCardsItemTimetracking {
    #[serde(rename = "originalEstimateSeconds")]
    pub original_estimate_seconds: Number,
    #[serde(rename = "timeSpentSeconds")]
    pub time_spent_seconds: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeDeckExportStacksItemCardsItemAssignedUsersItemParticipant {
    #[serde(rename = "uid")]
    pub uid: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeDeckExportStacksItemCardsItemAssignedUsersItem {
    #[serde(rename = "participant")]
    pub participant: NativeDeckExportStacksItemCardsItemAssignedUsersItemParticipant,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeDeckExportStacksItemCardsItem {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(
        rename = "description",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub description: Option<String>,
    #[serde(rename = "labels")]
    pub labels: Vec<NativeDeckExportStacksItemCardsItemLabelsItem>,
    #[serde(
        rename = "createdAt",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub created_at: Option<String>,
    #[serde(
        rename = "owner",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub owner: Option<NativeDeckExportStacksItemCardsItemOwner>,
    #[serde(
        rename = "comments",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub comments: Option<Vec<NativeDeckExportStacksItemCardsItemCommentsItem>>,
    #[serde(
        rename = "duedate",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub duedate: Option<String>,
    #[serde(
        rename = "timetracking",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub timetracking: Option<NativeDeckExportStacksItemCardsItemTimetracking>,
    #[serde(
        rename = "assignedUsers",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub assigned_users: Option<Vec<NativeDeckExportStacksItemCardsItemAssignedUsersItem>>,
    #[serde(
        rename = "done",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub done: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeDeckExportStacksItem {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "cards")]
    pub cards: Vec<NativeDeckExportStacksItemCardsItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeDeckExport {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "stacks")]
    pub stacks: Vec<NativeDeckExportStacksItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeKanboardExportBoard {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeKanboardExportColumnsItem {
    #[serde(rename = "title")]
    pub title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeKanboardExportSwimlanesItem {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeKanboardExportTasksItemSubtasksItem {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "status")]
    pub status: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeKanboardExportTasksItemCommentsItem {
    #[serde(rename = "comment")]
    pub comment: String,
    #[serde(
        rename = "username",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub username: Option<String>,
    #[serde(rename = "date_creation")]
    pub date_creation: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeKanboardExportTasksItemLinksItem {
    #[serde(rename = "id")]
    pub id: Number,
    #[serde(rename = "task_id")]
    pub task_id: Number,
    #[serde(rename = "label")]
    pub label: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeKanboardExportTasksItem {
    #[serde(rename = "id")]
    pub id: Number,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(
        rename = "description",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub description: Option<String>,
    #[serde(rename = "column_name")]
    pub column_name: String,
    #[serde(
        rename = "swimlane_name",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub swimlane_name: Option<String>,
    #[serde(
        rename = "date_creation",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub date_creation: Option<String>,
    #[serde(
        rename = "creator_username",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub creator_username: Option<String>,
    #[serde(rename = "tags")]
    pub tags: Vec<String>,
    #[serde(
        rename = "subtasks",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub subtasks: Option<Vec<NativeKanboardExportTasksItemSubtasksItem>>,
    #[serde(
        rename = "comments",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub comments: Option<Vec<NativeKanboardExportTasksItemCommentsItem>>,
    #[serde(
        rename = "date_due",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub date_due: Option<String>,
    #[serde(
        rename = "date_started",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub date_started: Option<String>,
    #[serde(
        rename = "owner_username",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub owner_username: Option<String>,
    #[serde(
        rename = "links",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub links: Option<Vec<NativeKanboardExportTasksItemLinksItem>>,
    #[serde(
        rename = "date_completed",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub date_completed: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeKanboardExport {
    #[serde(rename = "board")]
    pub board: NativeKanboardExportBoard,
    #[serde(rename = "columns")]
    pub columns: Vec<NativeKanboardExportColumnsItem>,
    #[serde(rename = "swimlanes")]
    pub swimlanes: Vec<NativeKanboardExportSwimlanesItem>,
    #[serde(rename = "tasks")]
    pub tasks: Vec<NativeKanboardExportTasksItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedDescription {
    #[serde(
        rename = "raw",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub raw: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedLinksStatus {
    #[serde(rename = "title")]
    pub title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedLinksAuthor {
    #[serde(rename = "title")]
    pub title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedLinksSprint {
    #[serde(rename = "href")]
    pub href: String,
    #[serde(rename = "title")]
    pub title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedLinksVersion {
    pub href: String,
    pub title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedLinksAssignee {
    #[serde(rename = "title")]
    pub title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedLinksResponsible {
    #[serde(rename = "title")]
    pub title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedLinksParent {
    #[serde(rename = "href")]
    pub href: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedLinks {
    #[serde(rename = "status")]
    pub status: NativeOpenprojectExportEmbeddedElementsItemFixedLinksStatus,
    #[serde(
        rename = "author",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub author: Option<NativeOpenprojectExportEmbeddedElementsItemFixedLinksAuthor>,
    #[serde(
        rename = "sprint",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub sprint: Option<NativeOpenprojectExportEmbeddedElementsItemFixedLinksSprint>,
    #[serde(default, deserialize_with = "super::validation::optional_object")]
    pub version: Option<NativeOpenprojectExportEmbeddedElementsItemFixedLinksVersion>,
    #[serde(
        rename = "assignee",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub assignee: Option<NativeOpenprojectExportEmbeddedElementsItemFixedLinksAssignee>,
    #[serde(
        rename = "responsible",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub responsible: Option<NativeOpenprojectExportEmbeddedElementsItemFixedLinksResponsible>,
    #[serde(
        rename = "parent",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub parent: Option<NativeOpenprojectExportEmbeddedElementsItemFixedLinksParent>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedEmbeddedActivitiesItemComment {
    #[serde(rename = "raw")]
    pub raw: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedEmbeddedActivitiesItemLinksUser {
    #[serde(
        rename = "title",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub title: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedEmbeddedActivitiesItemLinks {
    #[serde(rename = "user", deserialize_with = "super::validation::object")]
    pub user: NativeOpenprojectExportEmbeddedElementsItemFixedEmbeddedActivitiesItemLinksUser,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedEmbeddedActivitiesItem {
    #[serde(rename = "_type")]
    pub _type: String,
    #[serde(rename = "comment")]
    pub comment: NativeOpenprojectExportEmbeddedElementsItemFixedEmbeddedActivitiesItemComment,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "_links")]
    pub _links: NativeOpenprojectExportEmbeddedElementsItemFixedEmbeddedActivitiesItemLinks,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixedEmbedded {
    #[serde(rename = "activities")]
    pub activities: Vec<NativeOpenprojectExportEmbeddedElementsItemFixedEmbeddedActivitiesItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedElementsItemFixed {
    #[serde(rename = "id")]
    pub id: Number,
    #[serde(rename = "subject")]
    pub subject: String,
    #[serde(rename = "description")]
    pub description: NativeOpenprojectExportEmbeddedElementsItemFixedDescription,
    #[serde(
        rename = "createdAt",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub created_at: Option<String>,
    #[serde(rename = "_links")]
    pub _links: NativeOpenprojectExportEmbeddedElementsItemFixedLinks,
    #[serde(
        rename = "_embedded",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub _embedded: Option<NativeOpenprojectExportEmbeddedElementsItemFixedEmbedded>,
    #[serde(
        rename = "dueDate",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due_date: Option<String>,
    #[serde(
        rename = "startDate",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub start_date: Option<String>,
    #[serde(
        rename = "position",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub position: Option<Number>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedSchemasItemEntity {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedVersionsItemDescription {
    #[serde(rename = "raw")]
    pub raw: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedVersionsItem {
    #[serde(rename = "_type")]
    pub _type: String,
    #[serde(rename = "id")]
    pub id: Number,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(
        rename = "description",
        default,
        deserialize_with = "super::validation::optional_object"
    )]
    pub description: Option<NativeOpenprojectExportEmbeddedVersionsItemDescription>,
    #[serde(
        rename = "startDate",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub start_date: Option<String>,
    #[serde(
        rename = "endDate",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub end_date: Option<String>,
    #[serde(rename = "status")]
    pub status: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedSprintsItemLinksSelf {
    #[serde(rename = "href")]
    pub href: String,
    #[serde(rename = "title")]
    pub title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedSprintsItemLinksStatus {
    #[serde(rename = "href")]
    pub href: String,
    #[serde(rename = "title")]
    pub title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedSprintsItemLinks {
    #[serde(rename = "self")]
    pub self_link: NativeOpenprojectExportEmbeddedSprintsItemLinksSelf,
    #[serde(rename = "status")]
    pub status: NativeOpenprojectExportEmbeddedSprintsItemLinksStatus,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbeddedSprintsItem {
    #[serde(rename = "_type")]
    pub _type: String,
    #[serde(rename = "id")]
    pub id: Number,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(default, deserialize_with = "super::validation::optional_object")]
    pub description: Option<NativeOpenprojectExportEmbeddedVersionsItemDescription>,
    #[serde(
        rename = "startDate",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub start_date: Option<String>,
    #[serde(
        rename = "finishDate",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub finish_date: Option<String>,
    #[serde(rename = "_links")]
    pub _links: NativeOpenprojectExportEmbeddedSprintsItemLinks,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExportEmbedded {
    #[serde(rename = "elements")]
    pub elements: Vec<super::dynamic_fields::NativeOpenprojectExportEmbeddedElementsItem>,
    #[serde(
        rename = "schemas",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub schemas: Option<Vec<BTreeMap<String, NativeOpenprojectExportEmbeddedSchemasItemEntity>>>,
    #[serde(
        rename = "versions",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub versions: Option<Vec<NativeOpenprojectExportEmbeddedVersionsItem>>,
    #[serde(
        rename = "sprints",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub sprints: Option<Vec<NativeOpenprojectExportEmbeddedSprintsItem>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeOpenprojectExport {
    #[serde(rename = "_embedded")]
    pub _embedded: NativeOpenprojectExportEmbedded,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeGithubExportItemLabelsItem {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeGithubExportItem {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(
        rename = "body",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub body: Option<String>,
    #[serde(rename = "state")]
    pub state: String,
    #[serde(rename = "labels")]
    pub labels: Vec<NativeGithubExportItemLabelsItem>,
    #[serde(
        rename = "due_date",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due_date: Option<String>,
}
pub(super) type NativeGithubExport = Vec<NativeGithubExportItem>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeGitlabExportItemAuthor {
    #[serde(rename = "username")]
    pub username: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeGitlabExportItemNotesItemAuthor {
    // Wekan keeps comments after account deletion and then emits author: {}.
    #[serde(
        rename = "username",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub username: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeGitlabExportItemNotesItem {
    #[serde(rename = "body")]
    pub body: String,
    #[serde(rename = "author")]
    pub author: NativeGitlabExportItemNotesItemAuthor,
    #[serde(rename = "created_at")]
    pub created_at: String,
    #[serde(rename = "system")]
    pub system: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeGitlabExportItemAssigneesItem {
    #[serde(rename = "username")]
    pub username: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeGitlabExportItemIteration {
    #[serde(rename = "id")]
    pub id: Number,
    #[serde(rename = "iid")]
    pub iid: Number,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "state")]
    pub state: Number,
    #[serde(
        rename = "start_date",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub start_date: Option<String>,
    #[serde(
        rename = "due_date",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due_date: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeGitlabExportItemMilestone {
    #[serde(rename = "id")]
    pub id: Number,
    #[serde(rename = "iid")]
    pub iid: Number,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "state")]
    pub state: String,
    #[serde(
        rename = "start_date",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub start_date: Option<String>,
    #[serde(
        rename = "due_date",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due_date: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeGitlabExportItem {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(
        rename = "description",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub description: Option<String>,
    #[serde(rename = "state")]
    pub state: String,
    #[serde(rename = "labels")]
    pub labels: Vec<String>,
    #[serde(
        rename = "author",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub author: Option<NativeGitlabExportItemAuthor>,
    #[serde(
        rename = "created_at",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub created_at: Option<String>,
    #[serde(
        rename = "notes",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub notes: Option<Vec<NativeGitlabExportItemNotesItem>>,
    #[serde(
        rename = "due_date",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due_date: Option<String>,
    #[serde(
        rename = "assignees",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub assignees: Option<Vec<NativeGitlabExportItemAssigneesItem>>,
    #[serde(
        rename = "iteration",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub iteration: Option<NativeGitlabExportItemIteration>,
    #[serde(
        rename = "milestone",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub milestone: Option<NativeGitlabExportItemMilestone>,
    #[serde(
        rename = "closed_at",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub closed_at: Option<String>,
}
pub(super) type NativeGitlabExport = Vec<NativeGitlabExportItem>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeGiteaExportItemLabelsItem {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeGiteaExportItem {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(
        rename = "body",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub body: Option<String>,
    #[serde(rename = "state")]
    pub state: String,
    #[serde(rename = "labels")]
    pub labels: Vec<NativeGiteaExportItemLabelsItem>,
    #[serde(
        rename = "due_date",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due_date: Option<String>,
}
pub(super) type NativeGiteaExport = Vec<NativeGiteaExportItem>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeForgejoExportItemLabelsItem {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeForgejoExportItem {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(
        rename = "body",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub body: Option<String>,
    #[serde(rename = "state")]
    pub state: String,
    #[serde(rename = "labels")]
    pub labels: Vec<NativeForgejoExportItemLabelsItem>,
    #[serde(
        rename = "due_date",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due_date: Option<String>,
}
pub(super) type NativeForgejoExport = Vec<NativeForgejoExportItem>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeTrelloExportPrefs {
    #[serde(rename = "background")]
    pub background: String,
    #[serde(rename = "permissionLevel")]
    pub permission_level: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeTrelloExportLabelsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "color")]
    pub color: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeTrelloExportListsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "closed")]
    pub closed: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeTrelloExportCardsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(
        rename = "desc",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub desc: Option<String>,
    #[serde(rename = "idList")]
    pub id_list: String,
    #[serde(
        rename = "due",
        deserialize_with = "super::validation::required_nullable"
    )]
    pub due: Option<String>,
    #[serde(rename = "closed")]
    pub closed: bool,
    #[serde(rename = "idLabels")]
    pub id_labels: Vec<Option<String>>,
    #[serde(rename = "idMembers")]
    pub id_members: Vec<Never>,
    #[serde(rename = "idChecklists")]
    pub id_checklists: Vec<Never>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeTrelloExport {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "prefs")]
    pub prefs: NativeTrelloExportPrefs,
    #[serde(rename = "labels")]
    pub labels: Vec<NativeTrelloExportLabelsItem>,
    #[serde(rename = "lists")]
    pub lists: Vec<NativeTrelloExportListsItem>,
    #[serde(rename = "cards")]
    pub cards: Vec<NativeTrelloExportCardsItem>,
    #[serde(rename = "checklists")]
    pub checklists: Vec<Never>,
    #[serde(rename = "actions")]
    pub actions: Vec<Never>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportBoard {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedStatusStatusCategory {
    #[serde(rename = "key")]
    pub key: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedStatus {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(
        rename = "statusCategory",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub status_category: Option<NativeJiraExportIssuesItemFieldsFixedStatusStatusCategory>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedTimetracking {
    #[serde(rename = "timeSpentSeconds")]
    pub time_spent_seconds: Number,
    #[serde(
        rename = "originalEstimateSeconds",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub original_estimate_seconds: Option<Number>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedReporter {
    #[serde(rename = "displayName")]
    pub display_name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedSubtasksItemFieldsStatusStatusCategory {
    #[serde(rename = "key")]
    pub key: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedSubtasksItemFieldsStatus {
    #[serde(rename = "statusCategory")]
    pub status_category:
        NativeJiraExportIssuesItemFieldsFixedSubtasksItemFieldsStatusStatusCategory,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedSubtasksItemFields {
    #[serde(rename = "summary")]
    pub summary: String,
    #[serde(rename = "status")]
    pub status: NativeJiraExportIssuesItemFieldsFixedSubtasksItemFieldsStatus,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedSubtasksItem {
    #[serde(rename = "fields")]
    pub fields: NativeJiraExportIssuesItemFieldsFixedSubtasksItemFields,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedCommentCommentsItemAuthor {
    #[serde(
        rename = "name",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub name: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedCommentCommentsItem {
    #[serde(rename = "body")]
    pub body: String,
    #[serde(rename = "author", deserialize_with = "super::validation::object")]
    pub author: NativeJiraExportIssuesItemFieldsFixedCommentCommentsItemAuthor,
    #[serde(rename = "created")]
    pub created: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedComment {
    #[serde(rename = "comments")]
    pub comments: Vec<NativeJiraExportIssuesItemFieldsFixedCommentCommentsItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedIssuetype {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedFixVersionsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "released")]
    pub released: bool,
    #[serde(rename = "releaseDate")]
    pub release_date: String,
    #[serde(rename = "description")]
    pub description: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedAssignee {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixedParent {
    #[serde(rename = "key")]
    pub key: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItemFieldsFixed {
    #[serde(rename = "summary")]
    pub summary: String,
    #[serde(
        rename = "description",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub description: Option<String>,
    #[serde(rename = "status")]
    pub status: NativeJiraExportIssuesItemFieldsFixedStatus,
    #[serde(rename = "labels")]
    pub labels: Vec<String>,
    #[serde(
        rename = "timetracking",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub timetracking: Option<NativeJiraExportIssuesItemFieldsFixedTimetracking>,
    #[serde(
        rename = "created",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub created: Option<String>,
    #[serde(
        rename = "reporter",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub reporter: Option<NativeJiraExportIssuesItemFieldsFixedReporter>,
    #[serde(
        rename = "subtasks",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub subtasks: Option<Vec<NativeJiraExportIssuesItemFieldsFixedSubtasksItem>>,
    #[serde(
        rename = "comment",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub comment: Option<NativeJiraExportIssuesItemFieldsFixedComment>,
    #[serde(
        rename = "issuetype",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub issuetype: Option<NativeJiraExportIssuesItemFieldsFixedIssuetype>,
    #[serde(
        rename = "fixVersions",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub fix_versions: Option<Vec<NativeJiraExportIssuesItemFieldsFixedFixVersionsItem>>,
    #[serde(
        rename = "duedate",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub duedate: Option<String>,
    #[serde(
        rename = "assignee",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub assignee: Option<NativeJiraExportIssuesItemFieldsFixedAssignee>,
    #[serde(
        rename = "parent",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub parent: Option<NativeJiraExportIssuesItemFieldsFixedParent>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportIssuesItem {
    #[serde(rename = "key")]
    pub key: String,
    #[serde(rename = "fields")]
    pub fields: super::dynamic_fields::NativeJiraExportIssuesItemFields,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportWekanScrumMapping {
    #[serde(rename = "estimateFieldId")]
    pub estimate_field_id: String,
    #[serde(rename = "estimateUnit")]
    pub estimate_unit: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExportSchemaEntity {
    #[serde(rename = "type")]
    pub r#type: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeJiraExport {
    #[serde(rename = "board")]
    pub board: NativeJiraExportBoard,
    #[serde(rename = "issues")]
    pub issues: Vec<NativeJiraExportIssuesItem>,
    #[serde(
        rename = "wekanScrumMapping",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub wekan_scrum_mapping: Option<NativeJiraExportWekanScrumMapping>,
    #[serde(
        rename = "schema",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub schema: Option<BTreeMap<String, NativeJiraExportSchemaEntity>>,
    #[serde(
        rename = "names",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub names: Option<BTreeMap<String, String>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeAsanaExportDataItemMembershipsItemSection {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeAsanaExportDataItemMembershipsItem {
    #[serde(rename = "section")]
    pub section: NativeAsanaExportDataItemMembershipsItemSection,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeAsanaExportDataItemTagsItem {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeAsanaExportDataItemSubtasksItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "completed")]
    pub completed: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeAsanaExportDataItemStoriesItemCreatedBy {
    #[serde(
        rename = "name",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub name: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeAsanaExportDataItemStoriesItem {
    #[serde(rename = "resource_subtype")]
    pub resource_subtype: String,
    #[serde(rename = "text")]
    pub text: String,
    #[serde(rename = "created_by", deserialize_with = "super::validation::object")]
    pub created_by: NativeAsanaExportDataItemStoriesItemCreatedBy,
    #[serde(rename = "created_at")]
    pub created_at: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeAsanaExportDataItemAssignee {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeAsanaExportDataItemCustomFieldsItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(
        rename = "text_value",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub text_value: Option<String>,
    #[serde(
        rename = "number_value",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub number_value: Option<Number>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeAsanaExportDataItemParent {
    #[serde(rename = "gid")]
    pub gid: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeAsanaExportDataItem {
    #[serde(rename = "gid")]
    pub gid: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(
        rename = "notes",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub notes: Option<String>,
    #[serde(rename = "completed")]
    pub completed: bool,
    #[serde(
        rename = "created_at",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub created_at: Option<String>,
    #[serde(rename = "memberships")]
    pub memberships: Vec<NativeAsanaExportDataItemMembershipsItem>,
    #[serde(rename = "tags")]
    pub tags: Vec<NativeAsanaExportDataItemTagsItem>,
    #[serde(
        rename = "subtasks",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub subtasks: Option<Vec<NativeAsanaExportDataItemSubtasksItem>>,
    #[serde(
        rename = "stories",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub stories: Option<Vec<NativeAsanaExportDataItemStoriesItem>>,
    #[serde(
        rename = "due_on",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due_on: Option<String>,
    #[serde(
        rename = "start_on",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub start_on: Option<String>,
    #[serde(
        rename = "assignee",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub assignee: Option<NativeAsanaExportDataItemAssignee>,
    #[serde(
        rename = "custom_fields",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub custom_fields: Option<Vec<NativeAsanaExportDataItemCustomFieldsItem>>,
    #[serde(
        rename = "parent",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub parent: Option<NativeAsanaExportDataItemParent>,
    #[serde(
        rename = "completed_at",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub completed_at: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeAsanaExport {
    #[serde(rename = "data")]
    pub data: Vec<NativeAsanaExportDataItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeZenkitExportStagesItem {
    #[serde(rename = "name")]
    pub name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeZenkitExportItemsItemChecklistsItemItemsItem {
    #[serde(rename = "text")]
    pub text: String,
    #[serde(rename = "checked")]
    pub checked: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeZenkitExportItemsItemChecklistsItem {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "items")]
    pub items: Vec<NativeZenkitExportItemsItemChecklistsItemItemsItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeZenkitExportItemsItemCommentsItem {
    #[serde(rename = "text")]
    pub text: String,
    #[serde(
        rename = "author",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub author: Option<String>,
    #[serde(rename = "date")]
    pub date: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeZenkitExportItemsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(
        rename = "description",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub description: Option<String>,
    #[serde(rename = "stage_name")]
    pub stage_name: String,
    #[serde(rename = "tags")]
    pub tags: Vec<String>,
    #[serde(
        rename = "created_at",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub created_at: Option<String>,
    #[serde(
        rename = "checklists",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub checklists: Option<Vec<NativeZenkitExportItemsItemChecklistsItem>>,
    #[serde(
        rename = "comments",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub comments: Option<Vec<NativeZenkitExportItemsItemCommentsItem>>,
    #[serde(
        rename = "due",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub due: Option<String>,
    #[serde(
        rename = "start",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub start: Option<String>,
    #[serde(
        rename = "assignees",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub assignees: Option<Vec<String>>,
    #[serde(
        rename = "fields",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub fields: Option<BTreeMap<String, AttributeValue>>,
    #[serde(
        rename = "parent_id",
        default,
        deserialize_with = "super::validation::optional"
    )]
    pub parent_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeZenkitExport {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "stages")]
    pub stages: Vec<NativeZenkitExportStagesItem>,
    #[serde(rename = "items")]
    pub items: Vec<NativeZenkitExportItemsItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardHeader {
    #[serde(rename = "version")]
    pub version: Number,
    #[serde(rename = "date")]
    pub date: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardBoardProperties {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardBoardCardPropertiesItemOptionsItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "value")]
    pub value: String,
    #[serde(rename = "color")]
    pub color: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardBoardCardPropertiesItem {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "options")]
    pub options: Vec<FocalboardBoardCardPropertiesItemOptionsItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardBoard {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "teamId")]
    pub team_id: String,
    #[serde(rename = "channelId")]
    pub channel_id: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    #[serde(rename = "modifiedBy")]
    pub modified_by: String,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "minimumRole")]
    pub minimum_role: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "icon")]
    pub icon: String,
    #[serde(rename = "showDescription")]
    pub show_description: bool,
    #[serde(rename = "isTemplate")]
    pub is_template: bool,
    #[serde(rename = "templateVersion")]
    pub template_version: Number,
    #[serde(rename = "properties")]
    pub properties: FocalboardBoardProperties,
    #[serde(rename = "cardProperties")]
    pub card_properties: Vec<FocalboardBoardCardPropertiesItem>,
    #[serde(rename = "createAt")]
    pub create_at: Number,
    #[serde(rename = "updateAt")]
    pub update_at: Number,
    #[serde(rename = "deleteAt")]
    pub delete_at: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardViewBlockFieldsFilter {
    #[serde(rename = "operation")]
    pub operation: String,
    #[serde(rename = "filters")]
    pub filters: Vec<Never>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardViewBlockFieldsColumnWidths {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardViewBlockFieldsColumnCalculations {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardViewBlockFieldsKanbanCalculations {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardViewBlockFields {
    #[serde(rename = "viewType")]
    pub view_type: String,
    #[serde(rename = "groupById")]
    pub group_by_id: String,
    #[serde(rename = "sortOptions")]
    pub sort_options: Vec<Never>,
    #[serde(rename = "visiblePropertyIds")]
    pub visible_property_ids: Vec<String>,
    #[serde(rename = "visibleOptionIds")]
    pub visible_option_ids: Vec<String>,
    #[serde(rename = "hiddenOptionIds")]
    pub hidden_option_ids: Vec<String>,
    #[serde(rename = "collapsedOptionIds")]
    pub collapsed_option_ids: Vec<String>,
    #[serde(rename = "filter")]
    pub filter: FocalboardViewBlockFieldsFilter,
    #[serde(rename = "cardOrder")]
    pub card_order: Vec<String>,
    #[serde(rename = "columnWidths")]
    pub column_widths: FocalboardViewBlockFieldsColumnWidths,
    #[serde(rename = "columnCalculations")]
    pub column_calculations: FocalboardViewBlockFieldsColumnCalculations,
    #[serde(rename = "kanbanCalculations")]
    pub kanban_calculations: FocalboardViewBlockFieldsKanbanCalculations,
    #[serde(rename = "defaultTemplateId")]
    pub default_template_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardViewBlock {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: String,
    #[serde(rename = "boardId")]
    pub board_id: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    #[serde(rename = "modifiedBy")]
    pub modified_by: String,
    #[serde(rename = "schema")]
    pub schema: Number,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "fields")]
    pub fields: FocalboardViewBlockFields,
    #[serde(rename = "createAt")]
    pub create_at: Number,
    #[serde(rename = "updateAt")]
    pub update_at: Number,
    #[serde(rename = "deleteAt")]
    pub delete_at: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardCardBlockFields {
    #[serde(rename = "icon")]
    pub icon: String,
    #[serde(rename = "properties")]
    pub properties: BTreeMap<String, FocalboardPropertyValue>,
    #[serde(rename = "contentOrder")]
    pub content_order: Vec<String>,
    #[serde(rename = "isTemplate")]
    pub is_template: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardCardBlock {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: String,
    #[serde(rename = "boardId")]
    pub board_id: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    #[serde(rename = "modifiedBy")]
    pub modified_by: String,
    #[serde(rename = "schema")]
    pub schema: Number,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "fields")]
    pub fields: FocalboardCardBlockFields,
    #[serde(rename = "createAt")]
    pub create_at: Number,
    #[serde(rename = "updateAt")]
    pub update_at: Number,
    #[serde(rename = "deleteAt")]
    pub delete_at: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardCheckboxBlockFields {
    #[serde(rename = "value")]
    pub value: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardCheckboxBlock {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: String,
    #[serde(rename = "boardId")]
    pub board_id: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    #[serde(rename = "modifiedBy")]
    pub modified_by: String,
    #[serde(rename = "schema")]
    pub schema: Number,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "fields")]
    pub fields: FocalboardCheckboxBlockFields,
    #[serde(rename = "createAt")]
    pub create_at: Number,
    #[serde(rename = "updateAt")]
    pub update_at: Number,
    #[serde(rename = "deleteAt")]
    pub delete_at: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardCommentBlockFields {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardCommentBlock {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: String,
    #[serde(rename = "boardId")]
    pub board_id: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    #[serde(rename = "modifiedBy")]
    pub modified_by: String,
    #[serde(rename = "schema")]
    pub schema: Number,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "fields")]
    pub fields: FocalboardCommentBlockFields,
    #[serde(rename = "createAt")]
    pub create_at: Number,
    #[serde(rename = "updateAt")]
    pub update_at: Number,
    #[serde(rename = "deleteAt")]
    pub delete_at: Number,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardTextBlockFields {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FocalboardTextBlock {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: String,
    #[serde(rename = "boardId")]
    pub board_id: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    #[serde(rename = "modifiedBy")]
    pub modified_by: String,
    #[serde(rename = "schema")]
    pub schema: Number,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "fields")]
    pub fields: FocalboardTextBlockFields,
    #[serde(rename = "createAt")]
    pub create_at: Number,
    #[serde(rename = "updateAt")]
    pub update_at: Number,
    #[serde(rename = "deleteAt")]
    pub delete_at: Number,
}
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum FocalboardBlock {
    View(FocalboardViewBlock),
    Card(FocalboardCardBlock),
    Checkbox(FocalboardCheckboxBlock),
    Comment(FocalboardCommentBlock),
    Text(FocalboardTextBlock),
}
#[derive(Deserialize)]
#[serde(tag = "type", content = "data", deny_unknown_fields)]
pub(super) enum FocalboardRecord {
    #[serde(rename = "board")]
    Board(FocalboardBoard),
    #[serde(rename = "block")]
    Block(FocalboardBlock),
}
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum AttributeValue {
    String(String),
    Number(Number),
    Boolean(bool),
    List(Vec<AttributeValue>),
    Null(()),
}
#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum FocalboardPropertyValue {
    String(String),
    Strings(Vec<String>),
}
