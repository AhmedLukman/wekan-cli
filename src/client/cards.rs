use reqwest::{StatusCode, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Number;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{ClientError, WekanClient, transport::decode_success};

pub const CARD_COLORS: &[&str] = &[
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
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardSummary {
    #[serde(rename(deserialize = "_id", serialize = "card_id"))]
    pub card_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub swimlane_id: Option<String>,
    pub received_at: Option<String>,
    pub start_at: Option<String>,
    pub due_at: Option<String>,
    pub end_at: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub assignees: Vec<String>,
    pub sort: Option<Number>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum CardCustomFieldValue {
    String(String),
    Number(Number),
    Boolean(bool),
    Strings(Vec<String>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardCustomField {
    #[serde(rename(deserialize = "_id", serialize = "custom_field_id"))]
    pub custom_field_id: Option<String>,
    pub value: Option<CardCustomFieldValue>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CardStickerHighlight {
    Underline,
    Round,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CardSticker {
    pub icon: Option<String>,
    pub name: Option<String>,
    pub highlight: Option<CardStickerHighlight>,
    pub position: Option<Number>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardLocation {
    #[serde(rename(deserialize = "_id", serialize = "location_id"))]
    pub location_id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<Number>,
    pub longitude: Option<Number>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardDependency {
    pub card_id: String,
    #[serde(rename(deserialize = "type", serialize = "dependency_type"))]
    pub dependency_type: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardVote {
    pub question: String,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub positive: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub negative: Vec<String>,
    pub end: Option<String>,
    pub public: bool,
    pub allow_non_board_members: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardPoker {
    pub question: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub one: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub two: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub three: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub five: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub eight: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub thirteen: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub twenty: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub forty: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub one_hundred: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub unsure: Vec<String>,
    pub end: Option<String>,
    pub allow_non_board_members: Option<bool>,
    pub estimation: Option<Number>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct CardDocument {
    #[serde(rename(deserialize = "_id", serialize = "card_id"))]
    pub card_id: String,
    pub title: Option<String>,
    pub archived: bool,
    pub archived_at: Option<String>,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
    pub delete_batch_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_empty_string")]
    pub parent_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_empty_string")]
    pub list_id: Option<String>,
    pub swimlane_id: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_empty_string")]
    pub board_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_empty_string")]
    pub cover_id: Option<String>,
    pub color: Option<String>,
    pub created_at: String,
    pub modified_at: String,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub custom_fields: Vec<CardCustomField>,
    pub date_last_activity: String,
    pub description: Option<String>,
    pub requested_by: Option<String>,
    pub assigned_by: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub label_ids: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub members: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub assignees: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub requesters: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub assigners: Vec<String>,
    pub received_at: Option<String>,
    pub start_at: Option<String>,
    pub due_at: Option<String>,
    pub end_at: Option<String>,
    pub due_complete: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub stickers: Vec<CardSticker>,
    pub location_name: Option<String>,
    pub location_address: Option<String>,
    pub location_latitude: Option<Number>,
    pub location_longitude: Option<Number>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub locations: Vec<CardLocation>,
    pub spent_time: Option<Number>,
    pub is_overtime: Option<bool>,
    pub user_id: String,
    pub sort: Option<Number>,
    pub subtask_sort: Option<Number>,
    #[serde(rename(deserialize = "type", serialize = "card_type"))]
    pub card_type: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_empty_string")]
    pub linked_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_vec")]
    pub card_dependencies: Vec<CardDependency>,
    pub vote: Option<CardVote>,
    pub poker: Option<CardPoker>,
    #[serde(
        rename(deserialize = "targetId_gantt", serialize = "target_id_gantt"),
        default,
        deserialize_with = "deserialize_nullable_vec"
    )]
    pub target_id_gantt: Vec<String>,
    #[serde(
        rename(deserialize = "linkType_gantt", serialize = "link_type_gantt"),
        default,
        deserialize_with = "deserialize_nullable_vec"
    )]
    pub link_type_gantt: Vec<Number>,
    #[serde(
        rename(deserialize = "linkId_gantt", serialize = "link_id_gantt"),
        default,
        deserialize_with = "deserialize_nullable_vec"
    )]
    pub link_id_gantt: Vec<String>,
    pub card_number: Option<Number>,
    pub show_activities: bool,
    pub show_list_on_minicard: Option<bool>,
    pub show_checklist_at_minicard: Option<bool>,
    pub hide_finished_checklist_if_items_are_hidden: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCardRequest {
    pub title: String,
    pub swimlane_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignees: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCardRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spent_time: Option<Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_over_time: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignees: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_complete: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateCardResult {
    pub card_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateCardResult {
    pub card_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeleteCardResult {
    pub card_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdResponse {
    #[serde(rename = "_id")]
    id: String,
}

impl WekanClient {
    pub async fn board_list_cards(
        &self,
        board_id: &str,
        list_id: &str,
        token: &SecretString,
    ) -> Result<Vec<CardSummary>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.card_endpoint(board_id, list_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let cards = decode_success(&body, "board list card collection", true)?;
        validate_card_summaries(cards)
    }

    pub async fn card(
        &self,
        board_id: &str,
        list_id: &str,
        card_id: &str,
        token: &SecretString,
    ) -> Result<CardDocument, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.card_endpoint(board_id, list_id, Some(card_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        if body.is_empty() {
            return Err(ClientError::EmbeddedServer {
                http_status: StatusCode::OK,
                wekan_status_code: StatusCode::NOT_FOUND.as_u16(),
                server_error: Some("card-not-found".to_owned()),
                server_reason: Some("Card not found".to_owned()),
            });
        }
        let card = decode_success(&body, "card", true)?;
        validate_card_document(card)
    }

    pub async fn create_card(
        &self,
        board_id: &str,
        list_id: &str,
        request: &CreateCardRequest,
        token: &SecretString,
    ) -> Result<CreateCardResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.card_endpoint(board_id, list_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "card creation", true)?;
        require_non_empty(&response.id, "card creation", "card id")?;
        Ok(CreateCardResult {
            card_id: response.id,
        })
    }

    pub async fn update_card(
        &self,
        board_id: &str,
        list_id: &str,
        card_id: &str,
        request: &UpdateCardRequest,
        token: &SecretString,
    ) -> Result<UpdateCardResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.card_endpoint(board_id, list_id, Some(card_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "card update", true)?;
        require_non_empty(&response.id, "card update", "card id")?;
        Ok(UpdateCardResult {
            card_id: response.id,
        })
    }

    pub async fn delete_card(
        &self,
        board_id: &str,
        list_id: &str,
        card_id: &str,
        token: &SecretString,
    ) -> Result<DeleteCardResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .delete(self.card_endpoint(board_id, list_id, Some(card_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "card deletion", true)?;
        require_non_empty(&response.id, "card deletion", "card id")?;
        Ok(DeleteCardResult {
            card_id: response.id,
        })
    }

    fn card_endpoint(
        &self,
        board_id: &str,
        list_id: &str,
        card_id: Option<&str>,
    ) -> Result<reqwest::Url, ClientError> {
        let mut endpoint =
            self.server()
                .join("api/boards")
                .map_err(|error| ClientError::Protocol {
                    diagnostic: None,
                    message: format!("could not build the card endpoint: {error}"),
                    success_status_received: false,
                })?;
        let mut segments = endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                diagnostic: None,
                message: "could not add identifiers to the card endpoint".to_owned(),
                success_status_received: false,
            })?;
        segments.push(board_id);
        segments.push("lists");
        segments.push(list_id);
        segments.push("cards");
        if let Some(card_id) = card_id {
            segments.push(card_id);
        }
        drop(segments);
        Ok(endpoint)
    }
}

pub fn is_valid_card_color(value: &str) -> bool {
    CARD_COLORS.contains(&value)
        || value.len() == 7
            && value.starts_with('#')
            && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_card_summaries(cards: Vec<CardSummary>) -> Result<Vec<CardSummary>, ClientError> {
    for card in &cards {
        require_non_empty(&card.card_id, "board list card collection", "card id")?;
        validate_optional_id(
            card.swimlane_id.as_deref(),
            "board list card collection",
            "swimlane id",
        )?;
        validate_ids(&card.assignees, "board list card collection", "assignee id")?;
        for (field, value) in [
            ("receivedAt", card.received_at.as_deref()),
            ("startAt", card.start_at.as_deref()),
            ("dueAt", card.due_at.as_deref()),
            ("endAt", card.end_at.as_deref()),
        ] {
            validate_optional_date_time(value, field, "board list card collection")?;
        }
    }
    Ok(cards)
}

fn validate_card_document(card: CardDocument) -> Result<CardDocument, ClientError> {
    require_non_empty(&card.card_id, "card", "card id")?;
    require_non_empty(&card.swimlane_id, "card", "swimlane id")?;
    require_non_empty(&card.user_id, "card", "user id")?;
    if !matches!(
        card.card_type.as_str(),
        "cardType-card" | "cardType-linkedCard" | "cardType-linkedBoard" | "template-card"
    ) {
        return Err(protocol_error(format!(
            "the card response contained an invalid type `{}`",
            card.card_type
        )));
    }
    validate_date_time(&card.created_at, "createdAt", "card")?;
    validate_date_time(&card.modified_at, "modifiedAt", "card")?;
    validate_date_time(&card.date_last_activity, "dateLastActivity", "card")?;
    for (field, value) in [
        ("archivedAt", card.archived_at.as_deref()),
        ("deletedAt", card.deleted_at.as_deref()),
        ("receivedAt", card.received_at.as_deref()),
        ("startAt", card.start_at.as_deref()),
        ("dueAt", card.due_at.as_deref()),
        ("endAt", card.end_at.as_deref()),
        (
            "vote.end",
            card.vote.as_ref().and_then(|vote| vote.end.as_deref()),
        ),
        (
            "poker.end",
            card.poker.as_ref().and_then(|poker| poker.end.as_deref()),
        ),
    ] {
        validate_optional_date_time(value, field, "card")?;
    }
    for (field, value) in [
        ("parent id", card.parent_id.as_deref()),
        ("list id", card.list_id.as_deref()),
        ("board id", card.board_id.as_deref()),
        ("cover id", card.cover_id.as_deref()),
        ("linked id", card.linked_id.as_deref()),
    ] {
        validate_optional_id(value, "card", field)?;
    }
    if let Some(color) = &card.color
        && !color.is_empty()
        && !is_valid_card_color(color)
    {
        return Err(protocol_error(format!(
            "the card response contained an invalid color `{color}`"
        )));
    }
    for field in &card.custom_fields {
        validate_optional_id(field.custom_field_id.as_deref(), "card", "custom field id")?;
    }
    for location in &card.locations {
        require_non_empty(&location.location_id, "card", "location id")?;
    }
    for dependency in &card.card_dependencies {
        require_non_empty(&dependency.card_id, "card", "dependency card id")?;
        if let Some(dependency_type) = &dependency.dependency_type
            && !matches!(
                dependency_type.as_str(),
                "related-to" | "blocks" | "is-blocked-by" | "fixes" | "is-fixed-by"
            )
        {
            return Err(protocol_error(format!(
                "the card response contained an invalid dependency type `{dependency_type}`"
            )));
        }
    }
    for (values, field) in [
        (&card.label_ids, "label id"),
        (&card.members, "member id"),
        (&card.assignees, "assignee id"),
        (&card.requesters, "requester id"),
        (&card.assigners, "assigner id"),
        (&card.target_id_gantt, "Gantt target id"),
        (&card.link_id_gantt, "Gantt link id"),
    ] {
        validate_ids(values, "card", field)?;
    }
    if let Some(vote) = &card.vote {
        validate_ids(&vote.positive, "card", "positive voter id")?;
        validate_ids(&vote.negative, "card", "negative voter id")?;
    }
    if let Some(poker) = &card.poker {
        for (values, field) in [
            (&poker.one, "poker voter id"),
            (&poker.two, "poker voter id"),
            (&poker.three, "poker voter id"),
            (&poker.five, "poker voter id"),
            (&poker.eight, "poker voter id"),
            (&poker.thirteen, "poker voter id"),
            (&poker.twenty, "poker voter id"),
            (&poker.forty, "poker voter id"),
            (&poker.one_hundred, "poker voter id"),
            (&poker.unsure, "poker voter id"),
        ] {
            validate_ids(values, "card", field)?;
        }
    }
    Ok(card)
}

fn deserialize_nullable_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer).map(Option::unwrap_or_default)
}

fn deserialize_optional_non_empty_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(|value| value.filter(|value| !value.is_empty()))
}

fn validate_ids(values: &[String], operation: &str, field: &str) -> Result<(), ClientError> {
    for value in values {
        require_non_empty(value, operation, field)?;
    }
    Ok(())
}

fn validate_optional_id(
    value: Option<&str>,
    operation: &str,
    field: &str,
) -> Result<(), ClientError> {
    if let Some(value) = value {
        require_non_empty(value, operation, field)?;
    }
    Ok(())
}

fn require_non_empty(value: &str, operation: &str, field: &str) -> Result<(), ClientError> {
    if value.trim().is_empty() {
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

    use super::{CardDocument, CardSummary, is_valid_card_color};

    #[test]
    fn color_validation_matches_wekan_v11_06() {
        for value in ["white", "silver", "indigo", "#12aBcF"] {
            assert!(is_valid_card_color(value));
        }
        for value in ["", "belize", "#12345", "#12345g", "123456"] {
            assert!(!is_valid_card_color(value));
        }
    }

    #[test]
    fn response_objects_reject_unmapped_fields() {
        assert!(
            serde_json::from_value::<CardSummary>(json!({
                "_id": "card-1",
                "future": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<CardDocument>(json!({
                "_id": "card-1",
                "archived": false,
                "swimlaneId": "swimlane-1",
                "createdAt": "2030-01-02T03:04:05Z",
                "modifiedAt": "2030-01-02T03:04:05Z",
                "dateLastActivity": "2030-01-02T03:04:05Z",
                "userId": "user-1",
                "type": "cardType-card",
                "showActivities": false,
                "vote": {"future": true}
            }))
            .is_err()
        );
    }
}
