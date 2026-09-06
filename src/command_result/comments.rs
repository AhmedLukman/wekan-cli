use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CommentSummary {
    pub comment_id: String,
    pub text: String,
    pub author_id: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CommentCollectionSuccess {
    pub board_id: String,
    pub card_id: String,
    pub comments: Vec<CommentSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CommentDetail {
    pub comment_id: String,
    pub board_id: String,
    pub card_id: String,
    pub text: String,
    pub parent_id: Option<String>,
    pub created_at: String,
    pub modified_at: String,
    pub author_id: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CommentCreateSuccess {
    pub board_id: String,
    pub card_id: String,
    pub comment_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentDeleteMode {
    Hard,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct CommentDeleteSuccess {
    pub board_id: String,
    pub card_id: String,
    pub comment_id: String,
    pub deleted: bool,
    pub delete_mode: CommentDeleteMode,
}
