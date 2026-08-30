use reqwest::{StatusCode, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{ClientError, WekanClient, transport::decode_success};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommentSummary {
    #[serde(rename(deserialize = "_id", serialize = "comment_id"))]
    pub comment_id: String,
    #[serde(rename(deserialize = "comment", serialize = "text"))]
    pub text: String,
    #[serde(rename(deserialize = "authorId", serialize = "author_id"))]
    pub author_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CommentDocument {
    #[serde(rename(deserialize = "_id", serialize = "comment_id"))]
    pub comment_id: String,
    pub board_id: String,
    pub card_id: String,
    pub text: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_empty")]
    pub parent_id: Option<String>,
    pub created_at: String,
    pub modified_at: String,
    #[serde(rename(deserialize = "userId", serialize = "author_id"))]
    pub author_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CreateCommentRequest {
    #[serde(rename = "comment")]
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateCommentResult {
    pub comment_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeleteCommentResult {
    pub card_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdResponse {
    #[serde(rename = "_id")]
    id: String,
}

impl WekanClient {
    pub async fn card_comments(
        &self,
        board_id: &str,
        card_id: &str,
        token: &SecretString,
    ) -> Result<Vec<CommentSummary>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.comment_endpoint(board_id, card_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let comments = decode_success(&body, "card comment collection", true)?;
        validate_comment_summaries(comments)
    }

    pub async fn comment(
        &self,
        board_id: &str,
        card_id: &str,
        comment_id: &str,
        token: &SecretString,
    ) -> Result<CommentDocument, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.comment_endpoint(board_id, card_id, Some(comment_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        if body.is_empty() {
            return Err(ClientError::EmbeddedServer {
                http_status: StatusCode::OK,
                wekan_status_code: StatusCode::NOT_FOUND.as_u16(),
                server_error: Some("comment-not-found".to_owned()),
                server_reason: Some("Comment not found".to_owned()),
            });
        }
        let comment = validate_comment_document(decode_success(&body, "comment", true)?)?;
        if comment.board_id != board_id
            || comment.card_id != card_id
            || comment.comment_id != comment_id
        {
            return Err(protocol_error(
                "the comment response identifiers did not match the requested comment scope"
                    .to_owned(),
            ));
        }
        Ok(comment)
    }

    pub async fn create_comment(
        &self,
        board_id: &str,
        card_id: &str,
        request: &CreateCommentRequest,
        token: &SecretString,
    ) -> Result<CreateCommentResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.comment_endpoint(board_id, card_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "comment creation", true)?;
        require_non_empty(&response.id, "comment creation", "comment id")?;
        Ok(CreateCommentResult {
            comment_id: response.id,
        })
    }

    pub async fn delete_comment(
        &self,
        board_id: &str,
        card_id: &str,
        comment_id: &str,
        token: &SecretString,
    ) -> Result<DeleteCommentResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .delete(self.comment_endpoint(board_id, card_id, Some(comment_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "comment deletion", true)?;
        require_non_empty(&response.id, "comment deletion", "card id")?;
        Ok(DeleteCommentResult {
            card_id: response.id,
        })
    }

    fn comment_endpoint(
        &self,
        board_id: &str,
        card_id: &str,
        comment_id: Option<&str>,
    ) -> Result<reqwest::Url, ClientError> {
        let mut endpoint =
            self.server()
                .join("api/boards")
                .map_err(|error| ClientError::Protocol {
                    message: format!("could not build the comment endpoint: {error}"),
                    success_status_received: false,
                })?;
        let mut segments = endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                message: "could not add identifiers to the comment endpoint".to_owned(),
                success_status_received: false,
            })?;
        segments.push(board_id);
        segments.push("cards");
        segments.push(card_id);
        segments.push("comments");
        if let Some(comment_id) = comment_id {
            segments.push(comment_id);
        }
        drop(segments);
        Ok(endpoint)
    }
}

fn validate_comment_summaries(
    comments: Vec<CommentSummary>,
) -> Result<Vec<CommentSummary>, ClientError> {
    for comment in &comments {
        require_non_empty(&comment.comment_id, "card comment collection", "comment id")?;
        require_non_empty(&comment.text, "card comment collection", "text")?;
        require_non_empty(&comment.author_id, "card comment collection", "author id")?;
    }
    Ok(comments)
}

fn validate_comment_document(comment: CommentDocument) -> Result<CommentDocument, ClientError> {
    for (field, value) in [
        ("comment id", comment.comment_id.as_str()),
        ("board id", comment.board_id.as_str()),
        ("card id", comment.card_id.as_str()),
        ("text", comment.text.as_str()),
        ("author id", comment.author_id.as_str()),
    ] {
        require_non_empty(value, "comment", field)?;
    }
    if let Some(parent_id) = &comment.parent_id {
        require_non_empty(parent_id, "comment", "parent id")?;
    }
    validate_date_time(&comment.created_at, "createdAt")?;
    validate_date_time(&comment.modified_at, "modifiedAt")?;
    Ok(comment)
}

fn deserialize_optional_non_empty<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(|value| value.filter(|value| !value.is_empty()))
}

fn require_non_empty(value: &str, operation: &str, field: &str) -> Result<(), ClientError> {
    if value.trim().is_empty() {
        return Err(protocol_error(format!(
            "the {operation} response contained an empty {field}"
        )));
    }
    Ok(())
}

fn validate_date_time(value: &str, field: &str) -> Result<(), ClientError> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map(|_| ())
        .map_err(|_| {
            protocol_error(format!(
                "the comment response contained an invalid {field} date-time"
            ))
        })
}

fn protocol_error(message: String) -> ClientError {
    ClientError::Protocol {
        message,
        success_status_received: true,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{CommentDocument, CommentSummary, validate_comment_document};

    #[test]
    fn response_objects_reject_unmapped_fields() {
        assert!(
            serde_json::from_value::<CommentSummary>(json!({
                "_id": "comment-1",
                "comment": "Hello",
                "authorId": "user-1",
                "unexpected": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<CommentDocument>(json!({
                "_id": "comment-1",
                "boardId": "board-1",
                "cardId": "card-1",
                "text": "Hello",
                "createdAt": "2030-01-02T03:04:05Z",
                "modifiedAt": "2030-01-02T03:04:05Z",
                "userId": "user-1",
                "unexpected": true
            }))
            .is_err()
        );
    }

    #[test]
    fn empty_parent_ids_are_normalized_to_absence() {
        let comment = serde_json::from_value::<CommentDocument>(json!({
            "_id": "comment-1",
            "boardId": "board-1",
            "cardId": "card-1",
            "text": "Hello",
            "parentId": "",
            "createdAt": "2030-01-02T03:04:05Z",
            "modifiedAt": "2030-01-02T03:04:05Z",
            "userId": "user-1"
        }))
        .unwrap();

        assert_eq!(validate_comment_document(comment).unwrap().parent_id, None);
    }
}
