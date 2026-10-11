use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use super::{
    ClientError, WekanClient, decode_success, protocol_error, require_non_empty, validate_date_time,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveCardResult {
    pub card_id: String,
    pub archived: bool,
    pub archived_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestoreCardResult {
    pub card_id: String,
    pub archived: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ArchiveResponse {
    #[serde(rename = "_id")]
    card_id: String,
    archived: bool,
    archived_at: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreResponse {
    #[serde(rename = "_id")]
    card_id: String,
    archived: bool,
}

impl WekanClient {
    pub async fn archive_card(
        &self,
        board_id: &str,
        list_id: &str,
        card_id: &str,
        token: &SecretString,
    ) -> Result<ArchiveCardResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.card_action_endpoint(board_id, list_id, card_id, "archive")?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: ArchiveResponse = decode_success(&body, "card archive", true)?;
        validate_state_response(
            &response.card_id,
            card_id,
            response.archived,
            true,
            "card archive",
        )?;
        validate_date_time(&response.archived_at, "archivedAt", "card archive")?;
        Ok(ArchiveCardResult {
            card_id: response.card_id,
            archived: response.archived,
            archived_at: response.archived_at,
        })
    }

    pub async fn restore_card(
        &self,
        board_id: &str,
        list_id: &str,
        card_id: &str,
        token: &SecretString,
    ) -> Result<RestoreCardResult, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.card_action_endpoint(board_id, list_id, card_id, "unarchive")?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: RestoreResponse = decode_success(&body, "card restore", true)?;
        validate_state_response(
            &response.card_id,
            card_id,
            response.archived,
            false,
            "card restore",
        )?;
        Ok(RestoreCardResult {
            card_id: response.card_id,
            archived: response.archived,
        })
    }

    fn card_action_endpoint(
        &self,
        board_id: &str,
        list_id: &str,
        card_id: &str,
        action: &str,
    ) -> Result<reqwest::Url, ClientError> {
        let mut endpoint = self.card_endpoint(board_id, list_id, Some(card_id))?;
        endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                diagnostic: None,
                message: "could not add the action to the card endpoint".to_owned(),
                success_status_received: false,
            })?
            .push(action);
        Ok(endpoint)
    }
}

fn validate_state_response(
    actual_id: &str,
    requested_id: &str,
    archived: bool,
    expected_archived: bool,
    operation: &str,
) -> Result<(), ClientError> {
    require_non_empty(actual_id, operation, "card id")?;
    if actual_id != requested_id {
        return Err(protocol_error(format!(
            "the {operation} response reported a different card ID than requested"
        )));
    }
    if archived != expected_archived {
        return Err(protocol_error(format!(
            "the {operation} response contained an unexpected archived state"
        )));
    }
    Ok(())
}
