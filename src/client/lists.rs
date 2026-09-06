use reqwest::{StatusCode, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Number;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{ClientError, WekanClient, transport::decode_success};

pub const LIST_COLORS: &[&str] = &[
    "white",
    "green",
    "yellow",
    "orange",
    "red",
    "purple",
    "blue",
    "sky",
    "lime",
    "pink",
    "black",
    "silver",
    "peachpuff",
    "crimson",
    "plum",
    "darkgreen",
    "slateblue",
    "magenta",
    "gold",
    "navy",
    "gray",
    "saddlebrown",
    "paleturquoise",
    "mistyrose",
    "indigo",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ListSummary {
    #[serde(rename = "_id")]
    pub list_id: String,
    pub title: String,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub modified_at: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable")]
    pub cards_modified_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ListWipLimit {
    pub value: Number,
    pub enabled: bool,
    pub soft: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ListDocument {
    #[serde(rename(deserialize = "_id", serialize = "list_id"))]
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
    #[serde(
        rename(deserialize = "_updatedAt", serialize = "position_updated_at"),
        default
    )]
    pub position_updated_at: Option<String>,
    pub wip_limit: Option<ListWipLimit>,
    pub color: Option<String>,
    #[serde(rename(deserialize = "type", serialize = "list_type"))]
    pub list_type: String,
    pub width: Option<Number>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateListRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swimlane_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateListRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starred: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wip_limit: Option<ListWipLimit>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateListResult {
    pub list_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateListResult {
    pub list_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeleteListResult {
    pub list_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdResponse {
    #[serde(rename = "_id")]
    id: String,
}

impl WekanClient {
    pub async fn board_lists(
        &self,
        board_id: &str,
        token: &SecretString,
    ) -> Result<Vec<ListSummary>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.list_endpoint(board_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let lists = decode_success(&body, "board list collection", true)?;
        validate_list_summaries(lists)
    }

    pub async fn list(
        &self,
        board_id: &str,
        list_id: &str,
        token: &SecretString,
    ) -> Result<ListDocument, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.list_endpoint(board_id, Some(list_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        if body.is_empty() {
            return Err(ClientError::EmbeddedServer {
                http_status: StatusCode::OK,
                wekan_status_code: StatusCode::NOT_FOUND.as_u16(),
                server_error: Some("list-not-found".to_owned()),
                server_reason: Some("List not found".to_owned()),
            });
        }
        let list = decode_success(&body, "list", true)?;
        validate_list_document(list)
    }

    pub async fn create_list(
        &self,
        board_id: &str,
        request: &CreateListRequest,
        token: &SecretString,
    ) -> Result<CreateListResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.list_endpoint(board_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "list creation", true)?;
        require_non_empty(&response.id, "list creation", "list id")?;
        Ok(CreateListResult {
            list_id: response.id,
        })
    }

    pub async fn update_list(
        &self,
        board_id: &str,
        list_id: &str,
        request: &UpdateListRequest,
        token: &SecretString,
    ) -> Result<UpdateListResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.list_endpoint(board_id, Some(list_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "list update", true)?;
        require_non_empty(&response.id, "list update", "list id")?;
        Ok(UpdateListResult {
            list_id: response.id,
        })
    }

    pub async fn delete_list(
        &self,
        board_id: &str,
        list_id: &str,
        token: &SecretString,
    ) -> Result<DeleteListResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .delete(self.list_endpoint(board_id, Some(list_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "list deletion", true)?;
        require_non_empty(&response.id, "list deletion", "list id")?;
        Ok(DeleteListResult {
            list_id: response.id,
        })
    }

    fn list_endpoint(
        &self,
        board_id: &str,
        list_id: Option<&str>,
    ) -> Result<reqwest::Url, ClientError> {
        let mut endpoint =
            self.server()
                .join("api/boards")
                .map_err(|error| ClientError::Protocol {
                    diagnostic: None,
                    message: format!("could not build the list endpoint: {error}"),
                    success_status_received: false,
                })?;
        let mut segments = endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                diagnostic: None,
                message: "could not add identifiers to the list endpoint".to_owned(),
                success_status_received: false,
            })?;
        segments.push(board_id);
        segments.push("lists");
        if let Some(list_id) = list_id {
            segments.push(list_id);
        }
        drop(segments);
        Ok(endpoint)
    }
}

pub fn is_valid_list_color(value: &str) -> bool {
    LIST_COLORS.contains(&value)
        || value.len() == 7
            && value.starts_with('#')
            && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_list_summaries(lists: Vec<ListSummary>) -> Result<Vec<ListSummary>, ClientError> {
    for list in &lists {
        require_non_empty(&list.list_id, "board list collection", "list id")?;
        require_non_empty(&list.title, "board list collection", "title")?;
        validate_optional_date_time(
            list.modified_at.as_deref(),
            "modifiedAt",
            "board list collection",
        )?;
        validate_optional_date_time(
            list.cards_modified_at.as_deref(),
            "cardsModifiedAt",
            "board list collection",
        )?;
    }
    Ok(lists)
}

fn deserialize_required_nullable<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

fn validate_list_document(list: ListDocument) -> Result<ListDocument, ClientError> {
    require_non_empty(&list.list_id, "list", "list id")?;
    require_non_empty(&list.title, "list", "title")?;
    require_non_empty(&list.board_id, "list", "board id")?;
    validate_date_time(&list.created_at, "createdAt", "list")?;
    validate_date_time(&list.modified_at, "modifiedAt", "list")?;
    for (field, value) in [
        ("archivedAt", list.archived_at.as_deref()),
        ("deletedAt", list.deleted_at.as_deref()),
        ("updatedAt", list.updated_at.as_deref()),
        ("_updatedAt", list.position_updated_at.as_deref()),
    ] {
        validate_optional_date_time(value, field, "list")?;
    }
    if let Some(color) = &list.color
        && !color.is_empty()
        && !is_valid_list_color(color)
    {
        return Err(protocol_error(format!(
            "the list response contained an invalid color `{color}`"
        )));
    }
    if let Some(width) = &list.width {
        let valid = width
            .as_f64()
            .is_some_and(|width| (100.0..=1000.0).contains(&width));
        if !valid {
            return Err(protocol_error(
                "the list response contained a width outside 100 through 1000".to_owned(),
            ));
        }
    }
    Ok(list)
}

fn require_non_empty(value: &str, operation: &str, field: &str) -> Result<(), ClientError> {
    if value.is_empty() {
        return Err(protocol_error(format!(
            "the {operation} response contained an empty {field}"
        )));
    }
    Ok(())
}

fn validate_optional_date_time(
    value: Option<&str>,
    field: &str,
    operation: &str,
) -> Result<(), ClientError> {
    if let Some(value) = value {
        validate_date_time(value, field, operation)?;
    }
    Ok(())
}

fn validate_date_time(value: &str, field: &str, operation: &str) -> Result<(), ClientError> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map(|_| ())
        .map_err(|_| {
            protocol_error(format!(
                "the {operation} response contained an invalid {field} date-time"
            ))
        })
}

fn protocol_error(message: String) -> ClientError {
    ClientError::Protocol {
        diagnostic: None,
        message,
        success_status_received: true,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ListDocument, ListSummary, is_valid_list_color, validate_list_document};

    #[test]
    fn color_validation_matches_wekan_v11_06() {
        for value in ["white", "silver", "indigo", "#12aBcF"] {
            assert!(is_valid_list_color(value));
        }
        for value in ["", "belize", "#12345", "#12345g", "123456"] {
            assert!(!is_valid_list_color(value));
        }
    }

    #[test]
    fn list_response_preserves_blank_unset_color() {
        let list = serde_json::from_value::<ListDocument>(json!({
            "_id": "list-1",
            "title": "Todo",
            "archived": false,
            "boardId": "board-1",
            "createdAt": "2030-01-02T03:04:05Z",
            "modifiedAt": "2030-01-02T03:04:05Z",
            "type": "list",
            "color": ""
        }))
        .unwrap();

        let validated = validate_list_document(list).unwrap();
        assert_eq!(validated.color.as_deref(), Some(""));
    }

    #[test]
    fn response_objects_reject_unmapped_fields() {
        assert!(
            serde_json::from_value::<ListSummary>(json!({
                "_id": "list-1",
                "title": "Todo",
                "modifiedAt": null,
                "cardsModifiedAt": null,
                "unexpected": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<ListDocument>(json!({
                "_id": "list-1",
                "title": "Todo",
                "archived": false,
                "boardId": "board-1",
                "createdAt": "2030-01-02T03:04:05Z",
                "modifiedAt": "2030-01-02T03:04:05Z",
                "type": "list",
                "wipLimit": {
                    "value": 2,
                    "enabled": true,
                    "soft": false,
                    "unexpected": true
                }
            }))
            .is_err()
        );
    }

    #[test]
    fn list_summary_requires_nullable_timestamp_fields_to_be_present() {
        let missing_modified_at = serde_json::from_value::<ListSummary>(json!({
            "_id": "list-1",
            "title": "Todo",
            "cardsModifiedAt": null
        }));
        let missing_cards_modified_at = serde_json::from_value::<ListSummary>(json!({
            "_id": "list-1",
            "title": "Todo",
            "modifiedAt": null
        }));

        assert!(missing_modified_at.is_err());
        assert!(missing_cards_modified_at.is_err());
    }
}
