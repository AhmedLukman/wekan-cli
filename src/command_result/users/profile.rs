use serde::Serialize;
use serde_json::Number;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct UserProfileDetail {
    pub full_name: Option<String>,
    pub avatar_url: Option<String>,
    pub email_buffer: Option<Vec<Option<String>>>,
    pub show_desktop_drag_handles: Option<bool>,
    pub left_menu_collapsed: Option<bool>,
    pub collapsed_workspaces: Option<BTreeMap<String, bool>>,
    pub left_menu_width: Option<Number>,
    pub submit_on_enter: Option<bool>,
    pub open_many_cards_at_once: Option<bool>,
    pub all_boards_theme_tiles: Option<bool>,
    pub global_theme_color: Option<String>,
    pub global_theme_custom_colors: Option<Vec<Option<String>>>,
    pub ui_font: Option<String>,
    pub ui_font_size: Option<String>,
    pub ui_text_color: Option<String>,
    pub ui_text_bg_color: Option<String>,
    pub dismissed_announcement_version: Option<String>,
    pub card_maximized: Option<bool>,
    pub card_collapsed: Option<bool>,
    pub show_activities: Option<bool>,
    pub custom_fields_grid: Option<bool>,
    pub trello_api_saved: Option<bool>,
    pub hidden_minicard_label_text: Option<bool>,
    pub initials: Option<String>,
    pub board_workspaces_tree: Option<Vec<UserWorkspaceNodeDetail>>,
    pub board_workspace_assignments: Option<BTreeMap<String, String>>,
    pub invited_boards: Option<Vec<Option<String>>>,
    pub language: Option<String>,
    pub map_provider: Option<String>,
    pub move_and_copy_dialog: Option<BTreeMap<String, UserDialogOptionsDetail>>,
    pub move_checklist_dialog: Option<BTreeMap<String, UserDialogOptionsDetail>>,
    pub copy_checklist_dialog: Option<BTreeMap<String, UserDialogOptionsDetail>>,
    pub notifications: Option<Vec<UserProfileNotificationDetail>>,
    pub rescue_card_description: Option<bool>,
    pub show_cards_count_at: Option<Number>,
    pub start_day_of_week: Option<Number>,
    pub starred_boards: Option<Vec<Option<String>>>,
    pub starred_pages: Option<Vec<UserProfileStarredPageDetail>>,
    pub default_board_id: Option<String>,
    pub icode: Option<String>,
    pub board_view: Option<String>,
    pub list_sort_by: Option<String>,
    pub all_boards_sort_by: Option<String>,
    pub templates_board_id: Option<String>,
    pub card_templates_swimlane_id: Option<String>,
    pub list_templates_swimlane_id: Option<String>,
    pub board_templates_swimlane_id: Option<String>,
    pub sidebar_width: Option<Number>,
    pub list_widths: Option<BTreeMap<String, BTreeMap<String, Number>>>,
    pub list_constraints: Option<BTreeMap<String, BTreeMap<String, Number>>>,
    pub auto_width_boards: Option<BTreeMap<String, bool>>,
    pub board_sort_index: Option<BTreeMap<String, Number>>,
    pub fixed_list_width_boards: Option<BTreeMap<String, bool>>,
    pub fixed_list_widths: Option<BTreeMap<String, Number>>,
    pub swimlane_heights: Option<BTreeMap<String, BTreeMap<String, Number>>>,
    pub collapsed_lists: Option<BTreeMap<String, BTreeMap<String, bool>>>,
    pub collapsed_swimlanes: Option<BTreeMap<String, BTreeMap<String, bool>>>,
    pub collapsed_card_sections: Option<BTreeMap<String, BTreeMap<String, bool>>>,
    pub keyboard_shortcuts: Option<bool>,
    pub vertical_scrollbars: Option<bool>,
    pub show_week_of_year: Option<bool>,
    pub date_format: Option<String>,
    pub mobile_mode: Option<bool>,
    pub card_zoom: Option<Number>,
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
pub struct UserDialogOptionsDetail {
    pub board_id: String,
    pub swimlane_id: String,
    pub list_id: String,
    pub card_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
pub struct UserWorkspaceNodeDetail {
    pub id: Option<String>,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub children: Option<Vec<UserWorkspaceNodeDetail>>,
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
pub struct UserProfileNotificationDetail {
    pub activity: String,
    pub read: Option<String>,
}

#[derive(Clone, Debug, Serialize, Eq, PartialEq)]
pub struct UserProfileStarredPageDetail {
    pub url: String,
    pub title: String,
}
