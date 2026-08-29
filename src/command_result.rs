use serde::{Deserialize, Serialize};
use serde_json::Number;

#[derive(Debug, Eq, PartialEq)]
pub enum CommandSuccess {
    Cancelled(CancellationSuccess),
    Registration(AuthSuccess),
    Login(AuthSuccess),
    Logout(LogoutSuccess),
    AuthStatus(AuthStatusSuccess),
    UserCurrent(UserDetail),
    UserList(UserListSuccess),
    UserShown(UserDetail),
    UserCards(UserCardsSuccess),
    UserCreated(UserCreateSuccess),
    UserBoards(UserBoardsSuccess),
    UserOwnershipTaken(UserOwnershipSuccess),
    UserLoginChanged(UserLoginChangeSuccess),
    UserDeleted(UserDeleteSuccess),
    BoardList(BoardListSuccess),
    BoardCount(BoardCountSuccess),
    BoardShown(Box<BoardDetail>),
    BoardCreated(BoardCreateSuccess),
    BoardRenamed(BoardRenameSuccess),
    BoardDeleted(BoardDeleteSuccess),
    ListCollection(ListCollectionSuccess),
    ListShown(ListDetail),
    ListCreated(ListCreateSuccess),
    ListUpdated(ListUpdateSuccess),
    ListDeleted(ListDeleteSuccess),
    CardCollection(CardCollectionSuccess),
    CardShown(Box<CardDetail>),
    CardCreated(CardCreateSuccess),
    CardUpdated(CardUpdateSuccess),
    CardDeleted(CardDeleteSuccess),
    SwimlaneCollection(SwimlaneCollectionSuccess),
    SwimlaneShown(SwimlaneDetail),
    SwimlaneCreated(SwimlaneCreateSuccess),
    SwimlaneUpdated(SwimlaneUpdateSuccess),
    SwimlaneDeleted(SwimlaneDeleteSuccess),
    ProfileAdded(ProfileItem),
    ProfileList(ProfileListSuccess),
    ProfileShown(ProfileItem),
    ProfileUsed(ProfileItem),
    ProfileUpdated(ProfileItem),
    ProfileRemoved(ProfileRemoveSuccess),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DestructiveOperation {
    AuthLogout,
    ProfileRemove,
    UserTakeOwnership,
    UserDisableLogin,
    UserDelete,
    BoardDelete,
    ListDelete,
    CardDelete,
    SwimlaneDelete,
}

impl DestructiveOperation {
    pub const fn as_command(self) -> &'static str {
        match self {
            Self::AuthLogout => "auth logout",
            Self::ProfileRemove => "profile remove",
            Self::UserTakeOwnership => "user take-ownership",
            Self::UserDisableLogin => "user disable-login",
            Self::UserDelete => "user delete",
            Self::BoardDelete => "board delete",
            Self::ListDelete => "list delete",
            Self::CardDelete => "card delete",
            Self::SwimlaneDelete => "swimlane delete",
        }
    }
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CancellationSuccess {
    pub cancelled: bool,
    pub operation: DestructiveOperation,
}

impl CancellationSuccess {
    pub const fn new(operation: DestructiveOperation) -> Self {
        Self {
            cancelled: true,
            operation,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogoutScope {
    CurrentToken,
    AllTokens,
    LocalOnly,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct LogoutSuccess {
    pub server: String,
    pub profile: String,
    pub logout_scope: LogoutScope,
    pub remote_logout_completed: bool,
    pub credential_stored: bool,
    pub local_credential_removed: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AuthSuccess {
    pub server: String,
    pub profile: String,
    pub user_id: String,
    pub token_expires: String,
    pub credential_stored: bool,
    pub profile_created: bool,
    pub profile_active: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AuthStatusSuccess {
    pub server: String,
    pub profile: String,
    pub authenticated: bool,
    pub token_expires: String,
    pub credential_stored: bool,
    pub user: AuthStatusUser,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProfileItem {
    pub name: String,
    pub server: String,
    pub active: bool,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ProfileListSuccess {
    pub active_profile: Option<String>,
    pub profiles: Vec<ProfileItem>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ProfileRemoveSuccess {
    pub name: String,
    pub server: String,
    pub removed: bool,
    pub active_profile: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AuthStatusUser {
    pub user_id: String,
    pub username: Option<String>,
    pub full_name: Option<String>,
    pub is_admin: Option<bool>,
    pub emails: Vec<AuthStatusEmail>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AuthStatusEmail {
    pub address: Option<String>,
    pub verified: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserDetail {
    pub user_id: String,
    pub username: Option<String>,
    pub full_name: Option<String>,
    pub emails: Vec<UserEmail>,
    pub is_admin: Option<bool>,
    pub login_disabled: Option<bool>,
    pub authentication_method: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub last_connection_date: Option<String>,
    pub organizations: Vec<UserOrganization>,
    pub teams: Vec<UserTeam>,
    pub boards: Vec<UserBoardMembership>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserEmail {
    pub address: Option<String>,
    pub verified: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserOrganization {
    pub organization_id: Option<String>,
    pub display_name: Option<String>,
    pub is_admin: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserTeam {
    pub team_id: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserBoardMembership {
    pub board_id: String,
    pub is_active: Option<bool>,
    pub is_admin: Option<bool>,
    pub is_no_comments: Option<bool>,
    pub is_comment_only: Option<bool>,
    pub is_worker: Option<bool>,
    pub is_normal_assigned_only: Option<bool>,
    pub is_comment_assigned_only: Option<bool>,
    pub is_read_only: Option<bool>,
    pub is_read_assigned_only: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserSummary {
    pub user_id: String,
    pub username: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserListSuccess {
    pub users: Vec<UserSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UserCard {
    pub card_id: String,
    pub title: Option<String>,
    pub board_id: Option<String>,
    pub swimlane_id: Option<String>,
    pub list_id: Option<String>,
    pub due_at: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub members: Vec<String>,
    pub assignees: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserCardsSuccess {
    pub cards: Vec<UserCard>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BoardSummary {
    pub board_id: String,
    pub title: String,
}

pub type UserBoardSummary = BoardSummary;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BoardDetailMember {
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
#[serde(rename_all = "snake_case")]
pub struct BoardDetailWatcher {
    pub user_id: String,
    pub level: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BoardDetailLabel {
    pub label_id: String,
    pub name: Option<String>,
    pub color: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BoardDetailOrganization {
    pub org_id: String,
    pub org_display_name: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BoardDetailTeam {
    pub team_id: String,
    pub team_display_name: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BoardDetailDomain {
    pub domain: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BoardDetail {
    pub board_id: String,
    pub title: String,
    pub slug: Option<String>,
    pub archived: Option<bool>,
    pub archived_at: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: Option<String>,
    pub stars: Option<Number>,
    #[serde(default)]
    pub labels: Vec<BoardDetailLabel>,
    #[serde(default)]
    pub members: Vec<BoardDetailMember>,
    #[serde(default)]
    pub watchers: Vec<BoardDetailWatcher>,
    pub permission: Option<String>,
    #[serde(default)]
    pub orgs: Vec<BoardDetailOrganization>,
    #[serde(default)]
    pub teams: Vec<BoardDetailTeam>,
    #[serde(default)]
    pub domains: Vec<BoardDetailDomain>,
    #[serde(default)]
    pub import_usernames: Vec<String>,
    pub color: Option<String>,
    #[serde(default)]
    pub custom_theme_colors: Vec<String>,
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
    pub present_parent_task: Option<String>,
    pub received_at: Option<String>,
    pub start_at: Option<String>,
    pub due_at: Option<String>,
    pub end_at: Option<String>,
    pub spent_time: Option<Number>,
    pub is_overtime: Option<bool>,
    pub board_type: Option<String>,
    pub sort: Option<Number>,
    pub show_activities: Option<bool>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserBoardsSuccess {
    pub user_id: String,
    pub boards: Vec<UserBoardSummary>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserCreateWarning {
    #[serde(rename = "user_id_unavailable_in_wekan_v11_06")]
    UserIdUnavailableInWekanV1106,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserCreateSuccess {
    pub created: bool,
    pub username: String,
    pub email: String,
    pub user_id: Option<String>,
    pub warning: Option<UserCreateWarning>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserOwnershipSuccess {
    pub from_user_id: String,
    pub to_user_id: String,
    pub boards: Vec<UserBoardSummary>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UserLoginAction {
    Disabled,
    Enabled,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserLoginChangeSuccess {
    pub action: UserLoginAction,
    pub user: UserDetail,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UserDeleteSuccess {
    pub user_id: String,
    pub deleted: bool,
    pub deleted_current_user: bool,
    pub credential_stored: bool,
    pub local_credential_removed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardListScope {
    Active,
    Public,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BoardListSuccess {
    pub scope: BoardListScope,
    pub boards: Vec<BoardSummary>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BoardCountSuccess {
    pub private: u64,
    pub public: u64,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BoardCreateSuccess {
    pub board_id: String,
    pub default_swimlane_id: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BoardRenameSuccess {
    pub board_id: String,
    pub title: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct BoardDeleteSuccess {
    pub board_id: String,
    pub deleted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ListSummary {
    pub list_id: String,
    pub title: String,
    pub modified_at: Option<String>,
    pub cards_modified_at: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ListCollectionSuccess {
    pub board_id: String,
    pub lists: Vec<ListSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ListWipLimitDetail {
    pub value: Number,
    pub enabled: bool,
    pub soft: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ListDetail {
    pub list_id: String,
    pub title: String,
    pub starred: Option<bool>,
    pub archived: bool,
    pub archived_at: Option<String>,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
    pub delete_batch_id: Option<String>,
    pub board_id: String,
    pub swimlane_id: Option<String>,
    pub created_at: String,
    pub sort: Option<Number>,
    pub updated_at: Option<String>,
    pub modified_at: String,
    pub position_updated_at: Option<String>,
    pub wip_limit: Option<ListWipLimitDetail>,
    pub color: Option<String>,
    pub list_type: String,
    pub width: Option<Number>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ListCreateSuccess {
    pub board_id: String,
    pub list_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ListUpdatedField {
    Title,
    Color,
    Starred,
    WipLimit,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ListUpdateSuccess {
    pub board_id: String,
    pub list_id: String,
    pub updated_fields: Vec<ListUpdatedField>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ListDeleteMode {
    Soft,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ListDeleteSuccess {
    pub board_id: String,
    pub list_id: String,
    pub deleted: bool,
    pub delete_mode: ListDeleteMode,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardSummary {
    pub card_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub swimlane_id: Option<String>,
    pub received_at: Option<String>,
    pub start_at: Option<String>,
    pub due_at: Option<String>,
    pub end_at: Option<String>,
    pub assignees: Vec<String>,
    pub sort: Option<Number>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CardCollectionSuccess {
    pub board_id: String,
    pub list_id: String,
    pub cards: Vec<CardSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CardDetailCustomFieldValue {
    String(String),
    Number(Number),
    Boolean(bool),
    Strings(Vec<String>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailCustomField {
    pub custom_field_id: Option<String>,
    pub value: Option<CardDetailCustomFieldValue>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CardDetailStickerHighlight {
    Underline,
    Round,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailSticker {
    pub icon: Option<String>,
    pub name: Option<String>,
    pub highlight: Option<CardDetailStickerHighlight>,
    pub position: Option<Number>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailLocation {
    pub location_id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<Number>,
    pub longitude: Option<Number>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailDependency {
    pub card_id: String,
    pub dependency_type: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailVote {
    pub question: String,
    pub positive: Vec<String>,
    pub negative: Vec<String>,
    pub end: Option<String>,
    pub public: bool,
    pub allow_non_board_members: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetailPoker {
    pub question: Option<bool>,
    pub one: Vec<String>,
    pub two: Vec<String>,
    pub three: Vec<String>,
    pub five: Vec<String>,
    pub eight: Vec<String>,
    pub thirteen: Vec<String>,
    pub twenty: Vec<String>,
    pub forty: Vec<String>,
    pub one_hundred: Vec<String>,
    pub unsure: Vec<String>,
    pub end: Option<String>,
    pub allow_non_board_members: Option<bool>,
    pub estimation: Option<Number>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CardDetail {
    pub card_id: String,
    pub title: Option<String>,
    pub archived: bool,
    pub archived_at: Option<String>,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
    pub delete_batch_id: Option<String>,
    pub parent_id: Option<String>,
    pub list_id: Option<String>,
    pub swimlane_id: String,
    pub board_id: Option<String>,
    pub cover_id: Option<String>,
    pub color: Option<String>,
    pub created_at: String,
    pub modified_at: String,
    pub custom_fields: Vec<CardDetailCustomField>,
    pub date_last_activity: String,
    pub description: Option<String>,
    pub requested_by: Option<String>,
    pub assigned_by: Option<String>,
    pub label_ids: Vec<String>,
    pub members: Vec<String>,
    pub assignees: Vec<String>,
    pub requesters: Vec<String>,
    pub assigners: Vec<String>,
    pub received_at: Option<String>,
    pub start_at: Option<String>,
    pub due_at: Option<String>,
    pub end_at: Option<String>,
    pub due_complete: Option<bool>,
    pub stickers: Vec<CardDetailSticker>,
    pub location_name: Option<String>,
    pub location_address: Option<String>,
    pub location_latitude: Option<Number>,
    pub location_longitude: Option<Number>,
    pub locations: Vec<CardDetailLocation>,
    pub spent_time: Option<Number>,
    pub is_overtime: Option<bool>,
    pub user_id: String,
    pub sort: Option<Number>,
    pub subtask_sort: Option<Number>,
    pub card_type: String,
    pub linked_id: Option<String>,
    pub card_dependencies: Vec<CardDetailDependency>,
    pub vote: Option<CardDetailVote>,
    pub poker: Option<CardDetailPoker>,
    pub target_id_gantt: Vec<String>,
    pub link_type_gantt: Vec<Number>,
    pub link_id_gantt: Vec<String>,
    pub card_number: Option<Number>,
    pub show_activities: bool,
    pub show_list_on_minicard: Option<bool>,
    pub show_checklist_at_minicard: Option<bool>,
    pub hide_finished_checklist_if_items_are_hidden: Option<bool>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CardCreateSuccess {
    pub board_id: String,
    pub list_id: String,
    pub card_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardSubmittedField {
    Title,
    Sort,
    ParentId,
    Description,
    Color,
    LabelIds,
    RequestedBy,
    AssignedBy,
    ReceivedAt,
    StartAt,
    DueAt,
    EndAt,
    SpentTime,
    IsOverTime,
    Members,
    Assignees,
    DueComplete,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CardUpdateSuccess {
    pub board_id: String,
    pub list_id: String,
    pub card_id: String,
    pub submitted_fields: Vec<CardSubmittedField>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardDeleteMode {
    Hard,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CardDeleteSuccess {
    pub board_id: String,
    pub list_id: String,
    pub card_id: String,
    pub deleted: bool,
    pub delete_mode: CardDeleteMode,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneSummary {
    pub swimlane_id: String,
    pub title: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneCollectionSuccess {
    pub board_id: String,
    pub swimlanes: Vec<SwimlaneSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneDetail {
    pub swimlane_id: String,
    pub title: String,
    pub archived: bool,
    pub archived_at: Option<String>,
    pub board_id: String,
    pub created_at: String,
    pub sort: Option<Number>,
    pub color: Option<String>,
    pub updated_at: Option<String>,
    pub modified_at: String,
    pub swimlane_type: String,
    pub height: Option<Number>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneCreateSuccess {
    pub board_id: String,
    pub swimlane_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SwimlaneUpdatedField {
    Title,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneUpdateSuccess {
    pub board_id: String,
    pub swimlane_id: String,
    pub updated_fields: Vec<SwimlaneUpdatedField>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SwimlaneDeleteMode {
    Hard,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SwimlaneDeleteSuccess {
    pub board_id: String,
    pub swimlane_id: String,
    pub deleted: bool,
    pub delete_mode: SwimlaneDeleteMode,
}
