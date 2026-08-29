use reqwest::{StatusCode, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{ClientError, WekanClient, transport::decode_success};

pub const SWIMLANE_COLORS: &[&str] = &[
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
pub struct SwimlaneSummary {
    #[serde(rename = "_id")]
    pub swimlane_id: String,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct SwimlaneDocument {
    #[serde(rename(deserialize = "_id", serialize = "swimlane_id"))]
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
    #[serde(rename(deserialize = "type", serialize = "swimlane_type"))]
    pub swimlane_type: String,
    pub height: Option<Number>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSwimlaneRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Number>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UpdateSwimlaneRequest {
    pub title: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateSwimlaneResult {
    pub swimlane_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateSwimlaneResult {
    pub swimlane_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeleteSwimlaneResult {
    pub swimlane_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdResponse {
    #[serde(rename = "_id")]
    id: String,
}

impl WekanClient {
    pub async fn board_swimlanes(
        &self,
        board_id: &str,
        token: &SecretString,
    ) -> Result<Vec<SwimlaneSummary>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.swimlane_endpoint(board_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let swimlanes = decode_success(&body, "board swimlane collection", true)?;
        validate_swimlane_summaries(swimlanes)
    }

    pub async fn swimlane(
        &self,
        board_id: &str,
        swimlane_id: &str,
        token: &SecretString,
    ) -> Result<SwimlaneDocument, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.swimlane_endpoint(board_id, Some(swimlane_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        if body.is_empty() {
            return Err(ClientError::EmbeddedServer {
                http_status: StatusCode::OK,
                wekan_status_code: StatusCode::NOT_FOUND.as_u16(),
                server_error: Some("swimlane-not-found".to_owned()),
                server_reason: Some("Swimlane not found".to_owned()),
            });
        }
        let swimlane = decode_success(&body, "swimlane", true)?;
        validate_swimlane_document(swimlane)
    }

    pub async fn create_swimlane(
        &self,
        board_id: &str,
        request: &CreateSwimlaneRequest,
        token: &SecretString,
    ) -> Result<CreateSwimlaneResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.swimlane_endpoint(board_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "swimlane creation", true)?;
        require_non_empty(&response.id, "swimlane creation", "swimlane id")?;
        Ok(CreateSwimlaneResult {
            swimlane_id: response.id,
        })
    }

    pub async fn update_swimlane(
        &self,
        board_id: &str,
        swimlane_id: &str,
        request: &UpdateSwimlaneRequest,
        token: &SecretString,
    ) -> Result<UpdateSwimlaneResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.swimlane_endpoint(board_id, Some(swimlane_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "swimlane update", true)?;
        require_non_empty(&response.id, "swimlane update", "swimlane id")?;
        Ok(UpdateSwimlaneResult {
            swimlane_id: response.id,
        })
    }

    pub async fn delete_swimlane(
        &self,
        board_id: &str,
        swimlane_id: &str,
        token: &SecretString,
    ) -> Result<DeleteSwimlaneResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .delete(self.swimlane_endpoint(board_id, Some(swimlane_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "swimlane deletion", true)?;
        require_non_empty(&response.id, "swimlane deletion", "swimlane id")?;
        Ok(DeleteSwimlaneResult {
            swimlane_id: response.id,
        })
    }

    fn swimlane_endpoint(
        &self,
        board_id: &str,
        swimlane_id: Option<&str>,
    ) -> Result<reqwest::Url, ClientError> {
        let mut endpoint =
            self.server()
                .join("api/boards")
                .map_err(|error| ClientError::Protocol {
                    message: format!("could not build the swimlane endpoint: {error}"),
                    success_status_received: false,
                })?;
        let mut segments = endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                message: "could not add identifiers to the swimlane endpoint".to_owned(),
                success_status_received: false,
            })?;
        segments.push(board_id);
        segments.push("swimlanes");
        if let Some(swimlane_id) = swimlane_id {
            segments.push(swimlane_id);
        }
        drop(segments);
        Ok(endpoint)
    }
}

pub fn is_valid_swimlane_color(value: &str) -> bool {
    SWIMLANE_COLORS.contains(&value)
        || value.len() == 7
            && value.starts_with('#')
            && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_swimlane_summaries(
    swimlanes: Vec<SwimlaneSummary>,
) -> Result<Vec<SwimlaneSummary>, ClientError> {
    for swimlane in &swimlanes {
        require_non_empty(
            &swimlane.swimlane_id,
            "board swimlane collection",
            "swimlane id",
        )?;
        require_non_empty(&swimlane.title, "board swimlane collection", "title")?;
    }
    Ok(swimlanes)
}

fn validate_swimlane_document(swimlane: SwimlaneDocument) -> Result<SwimlaneDocument, ClientError> {
    require_non_empty(&swimlane.swimlane_id, "swimlane", "swimlane id")?;
    require_non_empty(&swimlane.title, "swimlane", "title")?;
    require_non_empty(&swimlane.board_id, "swimlane", "board id")?;
    require_non_empty(&swimlane.swimlane_type, "swimlane", "type")?;
    validate_date_time(&swimlane.created_at, "createdAt", "swimlane")?;
    validate_date_time(&swimlane.modified_at, "modifiedAt", "swimlane")?;
    for (field, value) in [
        ("archivedAt", swimlane.archived_at.as_deref()),
        ("updatedAt", swimlane.updated_at.as_deref()),
    ] {
        validate_optional_date_time(value, field, "swimlane")?;
    }
    if let Some(color) = &swimlane.color
        && !color.is_empty()
        && !is_valid_swimlane_color(color)
    {
        return Err(protocol_error(format!(
            "the swimlane response contained an invalid color `{color}`"
        )));
    }
    if let Some(height) = &swimlane.height {
        let valid = height
            .as_f64()
            .is_some_and(|height| height == -1.0 || (50.0..=2000.0).contains(&height));
        if !valid {
            return Err(protocol_error(
                "the swimlane response contained a height outside -1 or 50 through 2000".to_owned(),
            ));
        }
    }
    Ok(swimlane)
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
        message,
        success_status_received: true,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{SwimlaneDocument, SwimlaneSummary, is_valid_swimlane_color};

    #[test]
    fn color_validation_matches_wekan_v11_06() {
        for value in ["white", "silver", "indigo", "#12aBcF"] {
            assert!(is_valid_swimlane_color(value));
        }
        for value in ["", "belize", "#12345", "#12345g", "123456"] {
            assert!(!is_valid_swimlane_color(value));
        }
    }

    #[test]
    fn response_objects_reject_unmapped_fields() {
        assert!(
            serde_json::from_value::<SwimlaneSummary>(json!({
                "_id": "swimlane-1",
                "title": "Delivery",
                "unexpected": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<SwimlaneDocument>(json!({
                "_id": "swimlane-1",
                "title": "Delivery",
                "archived": false,
                "boardId": "board-1",
                "createdAt": "2030-01-02T03:04:05Z",
                "modifiedAt": "2030-01-02T03:04:05Z",
                "type": "swimlane",
                "unexpected": true
            }))
            .is_err()
        );
    }
}
