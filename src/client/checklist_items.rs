use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Number;

use super::{
    ClientError, WekanClient,
    checklists::{protocol_error, require_date, require_id},
    transport::decode_success,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ChecklistItemDocument {
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub assignee_id: Option<String>,
    #[serde(default, deserialize_with = "super::decoding::optional_non_null")]
    pub linked_card_id: Option<String>,
    pub due_at: Option<String>,
    #[serde(rename(deserialize = "_id", serialize = "item_id"))]
    pub item_id: String,
    pub title: String,
    pub sort: Number,
    pub is_finished: bool,
    pub checklist_id: String,
    pub card_id: String,
    /// Denormalized metadata; v12.57 REST card moves can leave the source board ID.
    pub board_id: Option<String>,
    pub user_id: Option<String>,
    pub created_at: Option<String>,
    pub modified_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ChecklistItemSortInput {
    Number(Number),
    String(String),
    Null,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ChecklistItemFinishedInput {
    Boolean(bool),
    String(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CreateChecklistItemRequest {
    #[serde(rename = "dueAt", skip_serializing_if = "Option::is_none")]
    pub due_at: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<ChecklistItemSortInput>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateChecklistItemRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_finished: Option<ChecklistItemFinishedInput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateChecklistItemResult {
    pub item_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateChecklistItemResult {
    pub item_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeleteChecklistItemResult {
    pub item_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdResponse {
    #[serde(rename = "_id")]
    id: String,
}

impl WekanClient {
    pub async fn checklist_item(
        &self,
        board_id: &str,
        card_id: &str,
        checklist_id: &str,
        item_id: &str,
        token: &SecretString,
    ) -> Result<ChecklistItemDocument, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.checklist_item_endpoint(
                        board_id,
                        card_id,
                        checklist_id,
                        Some(item_id),
                    )?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let item: ChecklistItemDocument = decode_success(&body, "checklist item", true)?;
        for (value, field) in [
            (&item.item_id, "item id"),
            (&item.checklist_id, "checklist id"),
            (&item.card_id, "card id"),
        ] {
            require_id(value, field)?;
        }
        if let Some(id) = &item.board_id {
            require_id(id, "board id")?;
        }
        if let Some(id) = &item.user_id {
            require_id(id, "creator user id")?;
        }
        if item.item_id != item_id || item.checklist_id != checklist_id || item.card_id != card_id {
            return Err(protocol_error(
                "the checklist item response identifiers did not match the requested scope",
            ));
        }
        require_date(&item.modified_at)?;
        if let Some(date) = &item.due_at {
            require_date(date)?;
        }
        if let Some(date) = &item.created_at {
            require_date(date)?;
        }
        Ok(item)
    }

    pub async fn create_checklist_item(
        &self,
        board_id: &str,
        card_id: &str,
        checklist_id: &str,
        request: &CreateChecklistItemRequest,
        token: &SecretString,
    ) -> Result<CreateChecklistItemResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.checklist_item_endpoint(board_id, card_id, checklist_id, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "checklist item creation", true)?;
        require_id(&response.id, "created item id")?;
        Ok(CreateChecklistItemResult {
            item_id: response.id,
        })
    }

    pub async fn update_checklist_item(
        &self,
        board_id: &str,
        card_id: &str,
        checklist_id: &str,
        item_id: &str,
        request: &UpdateChecklistItemRequest,
        token: &SecretString,
    ) -> Result<UpdateChecklistItemResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.checklist_item_endpoint(
                        board_id,
                        card_id,
                        checklist_id,
                        Some(item_id),
                    )?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "checklist item update", true)?;
        require_id(&response.id, "updated item id")?;
        if response.id != item_id {
            return Err(protocol_error(
                "Wekan reported a different updated checklist item ID than requested",
            ));
        }
        Ok(UpdateChecklistItemResult {
            item_id: response.id,
        })
    }

    pub async fn delete_checklist_item(
        &self,
        board_id: &str,
        card_id: &str,
        checklist_id: &str,
        item_id: &str,
        token: &SecretString,
    ) -> Result<DeleteChecklistItemResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .delete(self.checklist_item_endpoint(
                        board_id,
                        card_id,
                        checklist_id,
                        Some(item_id),
                    )?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: IdResponse = decode_success(&body, "checklist item deletion", true)?;
        require_id(&response.id, "deleted item id")?;
        if response.id != item_id {
            return Err(protocol_error(
                "Wekan reported a different deleted checklist item ID than requested",
            ));
        }
        Ok(DeleteChecklistItemResult {
            item_id: response.id,
        })
    }

    fn checklist_item_endpoint(
        &self,
        board_id: &str,
        card_id: &str,
        checklist_id: &str,
        item_id: Option<&str>,
    ) -> Result<reqwest::Url, ClientError> {
        let mut endpoint = self.checklist_endpoint(board_id, card_id, Some(checklist_id))?;
        let mut segments = endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                diagnostic: None,
                message: "could not add identifiers to the checklist item endpoint".to_owned(),
                success_status_received: false,
            })?;
        segments.push("items");
        if let Some(id) = item_id {
            segments.push(id);
        }
        drop(segments);
        Ok(endpoint)
    }
}
