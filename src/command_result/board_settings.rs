use crate::client::boards::{BoardDomain, card_settings::BoardCardSettings};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BoardViewSettingsSuccess {
    pub board_id: String,
    pub settings: crate::client::boards::presentation::BoardViewSettings,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BoardCardFieldOrderSuccess {
    pub board_id: String,
    pub settings: crate::client::boards::presentation::BoardCardFieldOrder,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BoardCardSettingField {
    AllowsSwimlaneDragging,
    AllowsListDragging,
    AllowsCardDragging,
    AllowsChecklistDragging,
    AllowsChecklistItemDragging,
    AllowsSubtaskDragging,
    AllowsMinicardCollapse,
    LabelsAboveTitleOnMinicard,
    AllowsChecklistDueDate,
    AllowsChecklistTitle,
    AskForMoveReason,
    AllowsCommentsOnMinicard,
    AllowsSpentTime,
    AllowsSpentTimeOnMinicard,
    AllowsSwimlaneNameOnMinicard,
    ShowLabelText,
    AllowsStickers,
    AllowsStickersOnMinicard,
    AllowsLocation,
    AllowsDependencies,
    AllowsDependenciesOnMinicard,
    AllowsFlowtime,
    AllowsPomodoro,
    AllowsVote,
    AllowsVoteOnMinicard,
    AllowsPoker,
    AllowsPokerOnMinicard,
    AllowsTextNotes,
    AllowsCommentCountOnMinicard,
    AllowsIssueTypeOnMinicard,
    AllowsCardColorOnCard,
    AllowsCardColorOnMinicard,
    AllowsParentCardOnCard,
    AllowsParentCardOnMinicard,
    AllowsLinkedCardOnCard,
    AllowsLinkedCardOnMinicard,
    AllowsCardButtonsOnCard,
    AllowsDescriptionBadgeOnMinicard,
    AllowsUnreadCommentsOnMinicard,
    AllowsCardCollapse,
    AllowsChecklistTitleOnMinicard,
    AllowsLabelTextOnCard,

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
    AllowsCardButtonsOnMinicard,
    AllowsDescriptionBadgeOnCard,
    AllowsUnreadCommentsOnCard,
    AllowsDueFromTitleOnCard,
    AllowsDueFromTitleOnMinicard,
    AllowsSimilarCardsOnCard,
    AllowsSimilarCardsOnMinicard,
    AllowsAtRiskOnCard,
    AllowsAtRiskOnMinicard,
    AllowsSuggestedMembersOnCard,
    AllowsForecastOnCard,
    AllowsForecastOnMinicard,
    AllowsChecklistFromDescriptionOnCard,
    AllowsActionItemsOnCard,
    AllowsWhiteboardOnCard,
    LabelsAboveTitleOnCard,
    AllowsChecklistDueDateOnMinicard,
    AllowsLocationOnMinicard,
    AllowsSwimlaneNameOnCard,
    AllowsFlowtimeOnMinicard,
    AllowsPomodoroOnMinicard,
    AllowsAttachmentListOnMinicard,
    AllowsTextNotesOnMinicard,
    AllowsCommentCountOnCard,
    AllowsActivitiesOnMinicard,
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
