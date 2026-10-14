use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Number;

use super::{
    CardDocument, ClientError, WekanClient, require_non_empty, validate_card_document,
    validate_optional_date_time, validate_optional_id,
};
use crate::client::transport::decode_success;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct SwimlaneCardSummary {
    #[serde(rename(deserialize = "_id", serialize = "card_id"))]
    pub card_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub list_id: Option<String>,
    pub received_at: Option<String>,
    pub start_at: Option<String>,
    pub due_at: Option<String>,
    pub end_at: Option<String>,
    pub assignees: Option<Vec<Option<String>>>,
    pub sort: Option<Number>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BoardCardCount {
    board_cards_count: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListCardCount {
    list_cards_count: u64,
}

impl WekanClient {
    pub async fn board_card_count(
        &self,
        board_id: &str,
        token: &SecretString,
    ) -> Result<u64, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.board_endpoint(board_id, Some("cards_count"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let count: BoardCardCount = decode_success(&body, "board card count", true)?;
        Ok(count.board_cards_count)
    }

    pub async fn list_card_count(
        &self,
        board_id: &str,
        list_id: &str,
        token: &SecretString,
    ) -> Result<u64, ClientError> {
        let mut endpoint = self.list_endpoint(board_id, Some(list_id))?;
        endpoint
            .path_segments_mut()
            .expect("HTTP endpoint")
            .push("cards_count");
        let body = self
            .execute_success_request(
                self.http
                    .get(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let count: ListCardCount = decode_success(&body, "list card count", true)?;
        Ok(count.list_cards_count)
    }

    pub async fn card_by_id(
        &self,
        card_id: &str,
        token: &SecretString,
    ) -> Result<CardDocument, ClientError> {
        let mut endpoint = self
            .server()
            .join("api/cards")
            .map_err(|_| super::protocol_error("could not build the card endpoint".into()))?;
        endpoint
            .path_segments_mut()
            .expect("HTTP endpoint")
            .push(card_id);
        let body = self
            .execute_success_request(
                self.http
                    .get(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        validate_card_document(decode_success(&body, "card lookup by ID", true)?)
    }

    pub async fn swimlane_cards(
        &self,
        board_id: &str,
        swimlane_id: &str,
        token: &SecretString,
    ) -> Result<Vec<SwimlaneCardSummary>, ClientError> {
        let mut endpoint = self.swimlane_endpoint(board_id, Some(swimlane_id))?;
        endpoint
            .path_segments_mut()
            .expect("HTTP endpoint")
            .push("cards");
        let body = self
            .execute_success_request(
                self.http
                    .get(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let cards: Vec<SwimlaneCardSummary> =
            decode_success(&body, "swimlane card collection", true)?;
        for card in &cards {
            require_non_empty(&card.card_id, "swimlane card collection", "card id")?;
            validate_optional_id(
                card.list_id.as_deref(),
                "swimlane card collection",
                "list id",
            )?;
            for id in card.assignees.iter().flatten().flatten() {
                require_non_empty(id, "swimlane card collection", "assignee id")?;
            }
            for (field, value) in [
                ("receivedAt", &card.received_at),
                ("startAt", &card.start_at),
                ("dueAt", &card.due_at),
                ("endAt", &card.end_at),
            ] {
                validate_optional_date_time(value.as_deref(), field, "swimlane card collection")?;
            }
        }
        Ok(cards)
    }
}
