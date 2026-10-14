use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

use super::super::{ClientError, WekanClient, transport::decode_success};

pub const LABEL_COLORS: &[&str] = &[
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

#[derive(Debug, Serialize)]
pub struct AddBoardLabelRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl WekanClient {
    pub async fn add_board_label(
        &self,
        board: &str,
        label: &AddBoardLabelRequest,
        token: &SecretString,
    ) -> Result<String, ClientError> {
        super::members::validate_id(board)?;
        let body = self
            .execute_success_request(
                self.http
                    .put(self.board_endpoint(board, Some("labels"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(&serde_json::json!({"label": label})),
            )
            .await?;
        let id: String = decode_success(&body, "board label addition", true)?;
        if id.is_empty() {
            return Err(ClientError::Protocol {
                diagnostic: None,
                message: "Wekan returned an empty label ID".into(),
                success_status_received: true,
            });
        }
        Ok(id)
    }
}
