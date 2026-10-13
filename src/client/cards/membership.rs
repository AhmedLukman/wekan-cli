use reqwest::{Method, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use super::{ClientError, WekanClient, require_non_empty, validate_ids};
use crate::client::{boards::members::validate_id, transport::decode_success};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CardMembershipResponse {
    #[serde(rename = "_id")]
    pub card_id: String,
    pub members: Vec<String>,
    pub assignees: Vec<String>,
}

impl WekanClient {
    pub async fn add_card_member(
        &self,
        board: &str,
        list: &str,
        card: &str,
        user: &str,
        token: &SecretString,
    ) -> Result<CardMembershipResponse, ClientError> {
        self.change_card_membership((board, list, card), user, "members", Method::POST, token)
            .await
    }
    pub async fn remove_card_member(
        &self,
        board: &str,
        list: &str,
        card: &str,
        user: &str,
        token: &SecretString,
    ) -> Result<CardMembershipResponse, ClientError> {
        self.change_card_membership((board, list, card), user, "members", Method::DELETE, token)
            .await
    }
    pub async fn add_card_assignee(
        &self,
        board: &str,
        list: &str,
        card: &str,
        user: &str,
        token: &SecretString,
    ) -> Result<CardMembershipResponse, ClientError> {
        self.change_card_membership((board, list, card), user, "assignees", Method::POST, token)
            .await
    }
    pub async fn remove_card_assignee(
        &self,
        board: &str,
        list: &str,
        card: &str,
        user: &str,
        token: &SecretString,
    ) -> Result<CardMembershipResponse, ClientError> {
        self.change_card_membership(
            (board, list, card),
            user,
            "assignees",
            Method::DELETE,
            token,
        )
        .await
    }

    async fn change_card_membership(
        &self,
        (board, list, card): (&str, &str, &str),
        user: &str,
        field: &str,
        method: Method,
        token: &SecretString,
    ) -> Result<CardMembershipResponse, ClientError> {
        for id in [board, list, card, user] {
            validate_id(id)?;
        }
        let mut endpoint = self.card_endpoint(board, list, Some(card))?;
        endpoint
            .path_segments_mut()
            .expect("the server URL supports path segments")
            .extend([field, user]);
        let body = self
            .execute_success_request(
                self.http
                    .request(method, endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: CardMembershipResponse = decode_success(&body, "card membership", true)?;
        require_non_empty(&response.card_id, "card membership", "card id")?;
        validate_ids(&response.members, "card membership", "member id")?;
        validate_ids(&response.assignees, "card membership", "assignee id")?;
        if response.card_id != card {
            return Err(ClientError::Protocol {
                diagnostic: None,
                message: "Wekan acknowledged a different membership card ID than requested".into(),
                success_status_received: true,
            });
        }
        Ok(response)
    }
}
