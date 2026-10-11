use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Number;

use super::{
    ClientError, WekanClient, datetime::validate_response_date, transport::decode_success,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChecklistSummary {
    #[serde(rename(deserialize = "_id", serialize = "checklist_id"))]
    pub checklist_id: String,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ChecklistItemSummary {
    #[serde(rename(deserialize = "_id", serialize = "item_id"))]
    pub item_id: String,
    pub title: String,
    pub is_finished: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ChecklistDocument {
    #[serde(rename(deserialize = "_id", serialize = "checklist_id"))]
    pub checklist_id: String,
    pub card_id: String,
    /// Denormalized metadata; v11.06 REST card moves can leave the source board ID.
    pub board_id: Option<String>,
    pub user_id: Option<String>,
    pub title: String,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub modified_at: String,
    pub sort: Number,
    pub hide_checked_checklist_items: Option<bool>,
    pub hide_all_checklist_items: Option<bool>,
    pub show_checklist_at_minicard: Option<bool>,
    pub items: Vec<ChecklistItemSummary>,
}

/// Exact JSON forms accepted for checklist creation's `items` field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ChecklistItemsInput {
    String(String),
    Array(Vec<String>),
    Null,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct CreateChecklistRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<ChecklistItemsInput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateChecklistResult {
    pub checklist_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeleteChecklistResult {
    pub checklist_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdResponse {
    #[serde(rename = "_id")]
    id: String,
}

impl WekanClient {
    pub async fn card_checklists(
        &self,
        board_id: &str,
        card_id: &str,
        token: &SecretString,
    ) -> Result<Vec<ChecklistSummary>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.checklist_endpoint(board_id, card_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let checklists: Vec<ChecklistSummary> =
            decode_success(&body, "card checklist collection", true)?;
        for checklist in &checklists {
            require_id(&checklist.checklist_id, "checklist id")?;
        }
        Ok(checklists)
    }

    pub async fn checklist(
        &self,
        board_id: &str,
        card_id: &str,
        checklist_id: &str,
        token: &SecretString,
    ) -> Result<ChecklistDocument, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.checklist_endpoint(board_id, card_id, Some(checklist_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let checklist: ChecklistDocument = decode_success(&body, "checklist", true)?;
        require_id(&checklist.checklist_id, "checklist id")?;
        require_id(&checklist.card_id, "card id")?;
        if let Some(id) = &checklist.board_id {
            require_id(id, "board id")?;
        }
        if let Some(id) = &checklist.user_id {
            require_id(id, "creator user id")?;
        }
        if checklist.checklist_id != checklist_id || checklist.card_id != card_id {
            return Err(protocol_error(
                "the checklist response identifiers did not match the requested scope",
            ));
        }
        for date in [&checklist.created_at, &checklist.modified_at]
            .into_iter()
            .chain(checklist.finished_at.iter())
        {
            require_date(date)?;
        }
        for item in &checklist.items {
            require_id(&item.item_id, "item id")?;
        }
        Ok(checklist)
    }

    pub async fn create_checklist(
        &self,
        board_id: &str,
        card_id: &str,
        request: &CreateChecklistRequest,
        token: &SecretString,
    ) -> Result<CreateChecklistResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.checklist_endpoint(board_id, card_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "checklist creation", true)?;
        require_id(&response.id, "created checklist id")?;
        Ok(CreateChecklistResult {
            checklist_id: response.id,
        })
    }

    pub async fn delete_checklist(
        &self,
        board_id: &str,
        card_id: &str,
        checklist_id: &str,
        token: &SecretString,
    ) -> Result<DeleteChecklistResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .delete(self.checklist_endpoint(board_id, card_id, Some(checklist_id))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "checklist deletion", true)?;
        require_id(&response.id, "deleted checklist id")?;
        if response.id != checklist_id {
            return Err(protocol_error(
                "Wekan reported a different deleted checklist ID than requested",
            ));
        }
        Ok(DeleteChecklistResult {
            checklist_id: response.id,
        })
    }

    pub(super) fn checklist_endpoint(
        &self,
        board_id: &str,
        card_id: &str,
        checklist_id: Option<&str>,
    ) -> Result<reqwest::Url, ClientError> {
        let mut endpoint =
            self.server()
                .join("api/boards")
                .map_err(|error| ClientError::Protocol {
                    diagnostic: None,
                    message: format!("could not build the checklist endpoint: {error}"),
                    success_status_received: false,
                })?;
        let mut segments = endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                diagnostic: None,
                message: "could not add identifiers to the checklist endpoint".to_owned(),
                success_status_received: false,
            })?;
        segments.extend([board_id, "cards", card_id, "checklists"]);
        if let Some(id) = checklist_id {
            segments.push(id);
        }
        drop(segments);
        Ok(endpoint)
    }
}

pub(super) fn protocol_error(message: &str) -> ClientError {
    ClientError::Protocol {
        diagnostic: None,
        message: message.to_owned(),
        success_status_received: true,
    }
}

pub(super) fn require_id(value: &str, field: &str) -> Result<(), ClientError> {
    if value.trim().is_empty() {
        Err(protocol_error(&format!(
            "the checklist response contained an empty {field}"
        )))
    } else {
        Ok(())
    }
}

pub(super) fn require_date(value: &str) -> Result<(), ClientError> {
    validate_response_date(value)
        .map_err(|()| protocol_error("the checklist response contained an invalid date-time"))
}
