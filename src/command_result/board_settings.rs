use crate::client::boards::{BoardDomain, card_settings::BoardCardSettings};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardCardSettingField {
    AllowsCardCounterList,
    CardAging,
    AllowsBoardMemberList,
    AllowsShowLists,
    AllowsAttachments,
    AllowsChecklists,
    AllowsComments,
    AllowsDescriptionTitle,
    AllowsDescriptionText,
    AllowsActivities,
    AllowsLabels,
    AllowsCreator,
    AllowsAssignee,
    AllowsMembers,
    AllowsRequestedBy,
    AllowsCardSortingByNumber,
    AllowsCardNumber,
    AllowsAssignedBy,
    AllowsReceivedDate,
    AllowsStartDate,
    AllowsEndDate,
    AllowsDueDate,
    AllowsSubtasks,
    AllowsCreatorOnMinicard,
    AllowsAttachmentsOnMinicard,
    AllowsChecklistsOnMinicard,
    AllowsChecklistAtMinicard,
    AllowsCoverAttachmentOnMinicard,
    AllowsBadgeAttachmentOnMinicard,
    AllowsCardSortingByNumberOnMinicard,
    AllowsCardNumberOnMinicard,
    AllowsDescriptionTitleOnMinicard,
    AllowsDescriptionTextOnMinicard,
    AllowsLabelsOnMinicard,
    AllowsAssigneeOnMinicard,
    AllowsMembersOnMinicard,
    AllowsRequestedByOnMinicard,
    AllowsAssignedByOnMinicard,
    AllowsReceivedDateOnMinicard,
    AllowsStartDateOnMinicard,
    AllowsEndDateOnMinicard,
    AllowsDueDateOnMinicard,
    AllowsSubtasksOnMinicard,
    AllowsShowListsOnMinicard,
    CardAgingDays1,
    CardAgingDays2,
    CardAgingDays3,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BoardCardSettingsSuccess {
    pub board_id: String,
    pub settings: BoardCardSettings,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BoardCardSettingsUpdateSuccess {
    pub board_id: String,
    pub settings: BoardCardSettings,
    pub submitted_fields: Vec<BoardCardSettingField>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardDomainAction {
    Add,
    Remove,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BoardDomainCollectionSuccess {
    pub board_id: String,
    pub domains: Vec<BoardDomain>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BoardDomainMutationSuccess {
    pub board_id: String,
    pub domain: String,
    pub action: BoardDomainAction,
    pub domains: Vec<BoardDomain>,
}
