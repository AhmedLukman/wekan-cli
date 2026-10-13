mod api;
mod auth;
mod boards;
mod cards;
mod checklist_items;
mod checklists;
mod comments;
mod custom_field_items;
mod custom_fields;
mod dependencies;
mod lists;
mod profiles;
mod swimlanes;
mod users;

pub use api::{
    ApiResponseSuccess, RawApiResponseData, RawResponseBody, RawResponseHeader, RawValueEncoding,
};
pub use auth::{
    AuthStatusEmail, AuthStatusSuccess, AuthStatusUser, AuthSuccess, LogoutScope, LogoutSuccess,
};
pub use boards::{
    BoardCountSuccess, BoardCreateSuccess, BoardDeleteSuccess, BoardDetail, BoardDetailDomain,
    BoardDetailLabel, BoardDetailMember, BoardDetailOrganization, BoardDetailTeam,
    BoardDetailWatcher, BoardListScope, BoardListSuccess, BoardRenameSuccess, BoardSummary,
    UserBoardSummary,
};
pub use cards::{
    CardArchiveSuccess, CardCollectionSuccess, CardCreateSuccess, CardDeleteMode,
    CardDeleteSuccess, CardDetail, CardDetailCustomField, CardDetailCustomFieldValue,
    CardDetailDependency, CardDetailLocation, CardDetailPoker, CardDetailSticker,
    CardDetailStickerHighlight, CardDetailVote, CardRestoreSuccess, CardSubmittedField,
    CardSummary, CardUpdateSuccess,
};
pub use cards::{CardCustomFieldQuerySuccess, CardCustomFieldSetSuccess};
pub use checklist_items::{
    ChecklistItemCreateSuccess, ChecklistItemDeleteMode, ChecklistItemDeleteSuccess,
    ChecklistItemDetail, ChecklistItemSubmittedField, ChecklistItemUpdateSuccess,
};
pub use checklists::{
    ChecklistCollectionSuccess, ChecklistCreateSuccess, ChecklistDeleteMode,
    ChecklistDeleteSuccess, ChecklistDetail, ChecklistItemSummary, ChecklistSummary,
};
pub use comments::{
    CommentCollectionSuccess, CommentCreateSuccess, CommentDeleteMode, CommentDeleteSuccess,
    CommentDetail, CommentSummary,
};
pub use custom_field_items::{
    CustomFieldItemCreateSuccess, CustomFieldItemDeleteMode, CustomFieldItemDeleteSuccess,
    CustomFieldItemSubmittedField, CustomFieldItemUpdateSuccess,
};
pub use custom_fields::{
    CustomFieldCollectionSuccess, CustomFieldCreateSuccess, CustomFieldDeleteMode,
    CustomFieldDeleteSuccess, CustomFieldDetail, CustomFieldDropdownItem, CustomFieldSettings,
    CustomFieldSubmittedField, CustomFieldSummary, CustomFieldUpdateSuccess,
};
pub use dependencies::{
    BoardDependencyCollectionSuccess, CardDependencyCollectionSuccess, DependencyCreateSuccess,
    DependencyDeleteSuccess, DependencyLine, DependencySubmittedField, DependencySummary,
    DependencyUpdateSuccess,
};
pub use lists::{
    ListCollectionSuccess, ListCreateSuccess, ListDeleteMode, ListDeleteSuccess, ListDetail,
    ListSummary, ListUpdateSuccess, ListUpdatedField, ListWipLimitDetail,
};
pub use profiles::{ProfileItem, ProfileListSuccess, ProfileRemoveSuccess};
pub use swimlanes::{
    SwimlaneCollectionSuccess, SwimlaneCreateSuccess, SwimlaneDeleteMode, SwimlaneDeleteSuccess,
    SwimlaneDetail, SwimlaneSummary, SwimlaneUpdateSuccess, SwimlaneUpdatedField,
};
pub use users::{
    UserBoardMembership, UserBoardsSuccess, UserCard, UserCardsSuccess, UserCreateSuccess,
    UserCreateWarning, UserDeleteSuccess, UserDetail, UserEmail, UserListSuccess, UserLoginAction,
    UserLoginChangeSuccess, UserOrganization, UserOwnershipSuccess, UserProfileDetail,
    UserProfileNotificationDetail, UserProfileStarredPageDetail, UserSessionData, UserSummary,
    UserTeam, UserWorkspaceNodeDetail,
};

use serde::Serialize;

pub mod attachments;
pub use attachments::*;

#[derive(Debug, Eq, PartialEq)]
pub enum CommandSuccess {
    BoardDependencies(BoardDependencyCollectionSuccess),
    CardDependencies(CardDependencyCollectionSuccess),
    DependencyCreated(DependencyCreateSuccess),
    DependencyUpdated(DependencyUpdateSuccess),
    DependencyDeleted(DependencyDeleteSuccess),
    AttachmentList(AttachmentCollectionSuccess),
    AttachmentShown(crate::client::attachments::AttachmentInfo),
    AttachmentUploaded(
        AttachmentMutationSuccess<crate::client::attachments::AttachmentUploadResponse>,
    ),
    BackgroundUploaded(
        AttachmentMutationSuccess<crate::client::attachments::BackgroundUploadResponse>,
    ),
    AttachmentCopied(AttachmentMutationSuccess<crate::client::attachments::AttachmentCopyResponse>),
    AttachmentMoved(AttachmentMutationSuccess<crate::client::attachments::AttachmentMoveResponse>),
    AttachmentDeleted(
        AttachmentMutationSuccess<crate::client::attachments::AttachmentDeleteResponse>,
    ),
    AttachmentDownloaded(AttachmentDownloadSuccess),
    ApiResponse(ApiResponseSuccess),
    Cancelled(CancellationSuccess),
    Registration(AuthSuccess),
    Login(AuthSuccess),
    Logout(LogoutSuccess),
    AuthStatus(AuthStatusSuccess),
    UserCurrent(UserDetail),
    UserList(UserListSuccess),
    UserShown(UserDetail),
    UserUpdated(UserDetail),
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
    CardArchived(CardArchiveSuccess),
    CardRestored(CardRestoreSuccess),
    CardDeleted(CardDeleteSuccess),
    ChecklistCollection(ChecklistCollectionSuccess),
    ChecklistShown(ChecklistDetail),
    ChecklistCreated(ChecklistCreateSuccess),
    ChecklistDeleted(ChecklistDeleteSuccess),
    ChecklistItemShown(ChecklistItemDetail),
    ChecklistItemCreated(ChecklistItemCreateSuccess),
    ChecklistItemUpdated(ChecklistItemUpdateSuccess),
    ChecklistItemDeleted(ChecklistItemDeleteSuccess),
    CustomFieldCollection(CustomFieldCollectionSuccess),
    CustomFieldShown(CustomFieldDetail),
    CustomFieldCreated(CustomFieldCreateSuccess),
    CustomFieldUpdated(CustomFieldUpdateSuccess),
    CustomFieldDeleted(CustomFieldDeleteSuccess),
    CustomFieldItemsCreated(CustomFieldItemCreateSuccess),
    CustomFieldItemUpdated(CustomFieldItemUpdateSuccess),
    CustomFieldItemDeleted(CustomFieldItemDeleteSuccess),
    CardCustomFieldSet(CardCustomFieldSetSuccess),
    CardCustomFieldQuery(CardCustomFieldQuerySuccess),
    CommentCollection(CommentCollectionSuccess),
    CommentShown(CommentDetail),
    CommentCreated(CommentCreateSuccess),
    CommentDeleted(CommentDeleteSuccess),
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
    DependencyDelete,
    AttachmentDelete,
    ApiRequest,
    AuthLogout,
    ProfileRemove,
    UserTakeOwnership,
    UserDisableLogin,
    UserDelete,
    BoardDelete,
    ListDelete,
    CardDelete,
    ChecklistDelete,
    ChecklistItemDelete,
    CustomFieldDelete,
    CustomFieldItemDelete,
    CommentDelete,
    SwimlaneDelete,
}

impl DestructiveOperation {
    pub const fn as_command(self) -> &'static str {
        match self {
            Self::DependencyDelete => "dependency delete",
            Self::AttachmentDelete => "attachment delete",
            Self::ApiRequest => "api request",
            Self::AuthLogout => "auth logout",
            Self::ProfileRemove => "profile remove",
            Self::UserTakeOwnership => "user take-ownership",
            Self::UserDisableLogin => "user disable-login",
            Self::UserDelete => "user delete",
            Self::BoardDelete => "board delete",
            Self::ListDelete => "list delete",
            Self::CardDelete => "card delete",
            Self::ChecklistDelete => "checklist delete",
            Self::ChecklistItemDelete => "checklist-item delete",
            Self::CustomFieldDelete => "custom-field delete",
            Self::CustomFieldItemDelete => "custom-field-item delete",
            Self::CommentDelete => "comment delete",
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
