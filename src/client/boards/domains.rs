use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

use super::{BoardDomain, ClientError, WekanClient};
use crate::client::transport::decode_success;

#[derive(Debug, Serialize)]
pub struct AddBoardDomainRequest<'a> {
    pub domain: &'a str,
}

impl WekanClient {
    pub async fn board_domains(
        &self,
        board: &str,
        token: &SecretString,
    ) -> Result<Vec<BoardDomain>, ClientError> {
        super::members::validate_id(board)?;
        let body = self
            .execute_success_request(
                self.http
                    .get(self.board_endpoint(board, Some("domains"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "board domain list", true)
    }

    pub async fn add_board_domain(
        &self,
        board: &str,
        domain: &str,
        token: &SecretString,
    ) -> Result<Vec<BoardDomain>, ClientError> {
        super::members::validate_id(board)?;
        let body = self
            .execute_success_request(
                self.http
                    .post(self.board_endpoint(board, Some("domains"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(&AddBoardDomainRequest { domain }),
            )
            .await?;
        decode_success(&body, "board domain add", true)
    }

    pub async fn remove_board_domain(
        &self,
        board: &str,
        domain: &str,
        token: &SecretString,
    ) -> Result<Vec<BoardDomain>, ClientError> {
        super::members::validate_id(board)?;
        if domain.is_empty() || matches!(domain, "." | "..") {
            return Err(ClientError::Protocol {
                diagnostic: None,
                message: "domain path values must not be empty or dot-only; no request was sent"
                    .into(),
                success_status_received: false,
            });
        }
        let mut endpoint = self.board_endpoint(board, Some("domains"))?;
        endpoint
            .path_segments_mut()
            .expect("server URL supports path segments")
            .push(domain);
        let body = self
            .execute_success_request(
                self.http
                    .delete(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "board domain remove", true)
    }
}
