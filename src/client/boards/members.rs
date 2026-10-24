use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

use super::{BoardSummary, ClientError, WekanClient, validate_board_summaries};
use crate::client::transport::decode_success;

/// Named roles take precedence over individual flags in Wekan v12.57.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardMemberPermissions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_admin: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_no_comments: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_comment_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_worker: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_normal_assigned_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_comment_assigned_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_read_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_read_assigned_only: Option<bool>,
}

#[derive(Serialize)]
struct AddRequest<'a> {
    action: &'static str,
    #[serde(flatten)]
    permissions: &'a BoardMemberPermissions,
}

#[derive(Serialize)]
struct RemoveRequest {
    action: &'static str,
}

impl WekanClient {
    pub async fn add_board_member(
        &self,
        board: &str,
        user: &str,
        permissions: &BoardMemberPermissions,
        token: &SecretString,
    ) -> Result<Vec<BoardSummary>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.board_member_endpoint(board, user, Some("add"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(&AddRequest {
                        action: "add",
                        permissions,
                    }),
            )
            .await?;
        validate_board_summaries(
            decode_success(&body, "board member add", true)?,
            "board member add",
        )
    }

    pub async fn remove_board_member(
        &self,
        board: &str,
        user: &str,
        token: &SecretString,
    ) -> Result<Vec<BoardSummary>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.board_member_endpoint(board, user, Some("remove"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(&RemoveRequest { action: "remove" }),
            )
            .await?;
        validate_board_summaries(
            decode_success(&body, "board member remove", true)?,
            "board member remove",
        )
    }

    pub async fn update_board_member(
        &self,
        board: &str,
        user: &str,
        permissions: &BoardMemberPermissions,
        token: &SecretString,
    ) -> Result<u64, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.board_member_endpoint(board, user, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(permissions),
            )
            .await?;
        decode_success(&body, "board member update", true)
    }

    fn board_member_endpoint(
        &self,
        board: &str,
        user: &str,
        suffix: Option<&str>,
    ) -> Result<reqwest::Url, ClientError> {
        for id in [board, user] {
            validate_id(id)?;
        }
        let mut endpoint = self.board_endpoint(board, Some("members"))?;
        let mut segments = endpoint
            .path_segments_mut()
            .expect("the server URL supports path segments");
        segments.push(user);
        if let Some(suffix) = suffix {
            segments.push(suffix);
        }
        drop(segments);
        Ok(endpoint)
    }
}

pub(crate) fn validate_id(id: &str) -> Result<(), ClientError> {
    if crate::client::trim_javascript_whitespace(id).is_empty() || matches!(id, "." | "..") {
        return Err(ClientError::Protocol {
            diagnostic: None,
            message: "membership identifiers must be nonblank and cannot be dot-only; no request was sent".into(),
            success_status_received: false,
        });
    }
    Ok(())
}
