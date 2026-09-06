mod api;
mod auth;
mod boards;
mod cards;
mod comments;
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
    CardCollectionSuccess, CardCreateSuccess, CardDeleteMode, CardDeleteSuccess, CardDetail,
    CardDetailCustomField, CardDetailCustomFieldValue, CardDetailDependency, CardDetailLocation,
    CardDetailPoker, CardDetailSticker, CardDetailStickerHighlight, CardDetailVote,
    CardSubmittedField, CardSummary, CardUpdateSuccess,
};
pub use comments::{
    CommentCollectionSuccess, CommentCreateSuccess, CommentDeleteMode, CommentDeleteSuccess,
    CommentDetail, CommentSummary,
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
    UserLoginChangeSuccess, UserOrganization, UserOwnershipSuccess, UserSummary, UserTeam,
};

use serde::Serialize;

#[derive(Debug, Eq, PartialEq)]
pub enum CommandSuccess {
    ApiResponse(ApiResponseSuccess),
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
    ApiRequest,
    AuthLogout,
    ProfileRemove,
    UserTakeOwnership,
    UserDisableLogin,
    UserDelete,
    BoardDelete,
    ListDelete,
    CardDelete,
    CommentDelete,
    SwimlaneDelete,
}

impl DestructiveOperation {
    pub const fn as_command(self) -> &'static str {
        match self {
            Self::ApiRequest => "api request",
            Self::AuthLogout => "auth logout",
            Self::ProfileRemove => "profile remove",
            Self::UserTakeOwnership => "user take-ownership",
            Self::UserDisableLogin => "user disable-login",
            Self::UserDelete => "user delete",
            Self::BoardDelete => "board delete",
            Self::ListDelete => "list delete",
            Self::CardDelete => "card delete",
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
