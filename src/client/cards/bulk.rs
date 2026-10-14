use reqwest::{StatusCode, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::client::{ClientError, WekanClient, decoding::decode_value, transport::decode_success};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BulkCreateCardInput {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub title: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "required_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub parent_id: Option<String>,
    #[serde(
        default,
        deserialize_with = "required_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub swimlane_id: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub members: Option<Value>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub assignees: Option<Value>,
}

fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}
fn required_string<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateCardsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swimlane_id: Option<String>,
    pub cards: Vec<BulkCreateCardInput>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BulkCreateCardResponse {
    pub index: usize,
    #[serde(default, rename = "_id", deserialize_with = "required_string")]
    pub card_id: Option<String>,
    #[serde(default, deserialize_with = "required_string")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BulkDeleteCardsResponse {
    pub deleted: Vec<String>,
    pub not_found: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BulkLabelUpdate {
    #[serde(rename = "_id")]
    pub card_id: String,
    pub label_ids: Vec<Option<String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BulkLabelsResponse {
    pub updated: Vec<BulkLabelUpdate>,
    pub not_found: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkLabelsRequest {
    pub card_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub add_label_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub remove_label_ids: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BulkLabelsError {
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("Wekan rejected labels absent from the board")]
    InvalidLabels {
        server_error: String,
        invalid_label_ids: Vec<String>,
    },
}

fn protocol(message: &str) -> ClientError {
    ClientError::Protocol {
        message: message.into(),
        diagnostic: None,
        success_status_received: true,
    }
}

// Compare multisets: duplicate requests and duplicate result records are meaningful.
fn correlate(
    requested: &[String],
    returned: impl Iterator<Item = String>,
) -> Result<(), ClientError> {
    let mut expected = requested.to_vec();
    let mut actual: Vec<_> = returned.collect();
    expected.sort();
    actual.sort();
    if expected != actual {
        return Err(protocol(
            "Wekan returned bulk card IDs that do not match the request",
        ));
    }
    Ok(())
}

impl WekanClient {
    pub async fn bulk_create_cards(
        &self,
        board: &str,
        list: &str,
        request: &BulkCreateCardsRequest,
        token: &SecretString,
    ) -> Result<Vec<BulkCreateCardResponse>, ClientError> {
        for id in [board, list] {
            crate::client::boards::members::validate_id(id)?;
        }
        let mut endpoint = self.card_endpoint(board, list, None)?;
        endpoint.path_segments_mut().expect("base URL").push("bulk");
        let body = self
            .execute_success_request(
                self.http
                    .post(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let results: Vec<BulkCreateCardResponse> =
            decode_success(&body, "bulk card creation", true)?;
        if results.len() != request.cards.len()
            || results.iter().enumerate().any(|(i, result)| {
                result.index != i
                    || result.card_id.is_some() == result.error.is_some()
                    || result.card_id.as_ref().is_some_and(String::is_empty)
            })
        {
            return Err(protocol(
                "Wekan returned invalid bulk creation indices or outcomes",
            ));
        }
        Ok(results)
    }

    pub async fn bulk_delete_cards(
        &self,
        board: &str,
        card_ids: &[String],
        token: &SecretString,
    ) -> Result<BulkDeleteCardsResponse, ClientError> {
        let endpoint = self.bulk_card_endpoint(board, "bulk")?;
        let body = self
            .execute_success_request(
                self.http
                    .delete(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(&serde_json::json!({"cardIds": card_ids})),
            )
            .await?;
        let result: BulkDeleteCardsResponse = decode_success(&body, "bulk card deletion", true)?;
        correlate(
            card_ids,
            result.deleted.iter().chain(&result.not_found).cloned(),
        )?;
        Ok(result)
    }

    pub async fn bulk_card_labels(
        &self,
        board: &str,
        request: &BulkLabelsRequest,
        token: &SecretString,
    ) -> Result<BulkLabelsResponse, BulkLabelsError> {
        let endpoint = self.bulk_card_endpoint(board, "labels")?;
        let body = self
            .execute_success_request_with_error(
                self.http
                    .post(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
                label_error,
            )
            .await?;
        let result: BulkLabelsResponse = decode_success(&body, "bulk card labels", true)?;
        correlate(
            &request.card_ids,
            result
                .updated
                .iter()
                .map(|item| item.card_id.clone())
                .chain(result.not_found.iter().cloned()),
        )?;
        Ok(result)
    }

    fn bulk_card_endpoint(&self, board: &str, action: &str) -> Result<reqwest::Url, ClientError> {
        crate::client::boards::members::validate_id(board)?;
        let mut endpoint = self.board_endpoint(board, Some("cards"))?;
        endpoint.path_segments_mut().expect("base URL").push(action);
        Ok(endpoint)
    }
}

fn label_error(status: StatusCode, body: &[u8]) -> Result<Option<BulkLabelsError>, ClientError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "camelCase")]
    struct InvalidLabels {
        error: String,
        invalid_label_ids: Vec<String>,
    }
    if status == StatusCode::BAD_REQUEST {
        if let Ok(value) = serde_json::from_slice::<Value>(body) {
            if value.get("invalidLabelIds").is_some() {
                let error: InvalidLabels = decode_value(value, "bulk label error", false)?;
                return Ok(Some(BulkLabelsError::InvalidLabels {
                    server_error: error.error,
                    invalid_label_ids: error.invalid_label_ids,
                }));
            }
        }
    }
    Ok(None)
}
