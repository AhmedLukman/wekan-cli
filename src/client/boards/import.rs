use std::{collections::BTreeMap, time::Duration};

use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::header::{ACCEPT, CONTENT_LENGTH, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;
use tokio_util::io::ReaderStream;

use super::{ClientError, WekanClient, require_non_empty};
use crate::client::imports::PreparedImportRequest;
use crate::client::transport::decode_success;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportSource {
    Wekan,
    Trello,
    Csv,
    Jira,
    Kanboard,
    Excel,
    Deck,
    Openproject,
    Github,
    Gitlab,
    Gitea,
    Forgejo,
    Asana,
    Zenkit,
}

impl ImportSource {
    pub const ALL: [Self; 14] = [
        Self::Wekan,
        Self::Trello,
        Self::Csv,
        Self::Jira,
        Self::Kanboard,
        Self::Excel,
        Self::Deck,
        Self::Openproject,
        Self::Github,
        Self::Gitlab,
        Self::Gitea,
        Self::Forgejo,
        Self::Asana,
        Self::Zenkit,
    ];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wekan => "wekan",
            Self::Trello => "trello",
            Self::Csv => "csv",
            Self::Jira => "jira",
            Self::Kanboard => "kanboard",
            Self::Excel => "excel",
            Self::Deck => "deck",
            Self::Openproject => "openproject",
            Self::Github => "github",
            Self::Gitlab => "gitlab",
            Self::Gitea => "gitea",
            Self::Forgejo => "forgejo",
            Self::Asana => "asana",
            Self::Zenkit => "zenkit",
        }
    }
}

impl std::str::FromStr for ImportSource {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|source| source.as_str() == value)
            .ok_or("unsupported import source")
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardImportRequest {
    pub board: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members_mapping: Option<BTreeMap<String, String>>,
}

impl BoardImportRequest {
    pub fn excel(bytes: &[u8], members_mapping: Option<BTreeMap<String, String>>) -> Self {
        Self {
            board: serde_json::json!({"excelBase64": STANDARD.encode(bytes)}),
            members_mapping,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardImportResponse {
    #[serde(rename = "_id")]
    pub board_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ZipImportCounts {
    pub swimlanes: u64,
    pub lists: u64,
    pub cards: u64,
    pub checklists: u64,
    pub comments: u64,
    pub attachments: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZipImportResponse {
    pub ok: bool,
    pub counts: ZipImportCounts,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ZipImportTarget {
    pub board_id: String,
    pub fields: Vec<String>,
    pub swimlane_id: Option<String>,
    pub list_id: Option<String>,
    pub card_id: Option<String>,
    pub checklist_id: Option<String>,
}

impl WekanClient {
    /// Submit a typed JSON wrapper containing the complete source document.
    pub async fn import_board(
        &self,
        source: Option<ImportSource>,
        request: PreparedImportRequest<BoardImportRequest>,
        token: &SecretString,
        timeout: Option<Duration>,
    ) -> Result<BoardImportResponse, ClientError> {
        let path = source.map_or_else(
            || "api/boards/import".to_owned(),
            |s| format!("api/boards/import/{}", s.as_str()),
        );
        let mut request = self
            .http
            .post(self.boards_api_endpoint(&path, "board import")?)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .bearer_auth(token.expose_secret())
            .body(request.into_body());
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        let body = self.execute_success_request(request).await?;
        let response: BoardImportResponse = decode_success(&body, "board import", true)?;
        require_non_empty(&response.board_id, "board import", "board id")?;
        Ok(response)
    }

    pub async fn import_zip(
        &self,
        target: &ZipImportTarget,
        reader: impl AsyncRead + Send + Unpin + 'static,
        length: u64,
        token: &SecretString,
        query_token: bool,
        timeout: Option<Duration>,
    ) -> Result<ZipImportResponse, ClientError> {
        let mut url = self.boards_api_endpoint("api/import/zip", "ZIP import")?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("boardId", &target.board_id);
            if !target.fields.is_empty() {
                query.append_pair("fields", &target.fields.join(","));
            }
            for (key, value) in [
                ("swimlaneId", &target.swimlane_id),
                ("listId", &target.list_id),
                ("cardId", &target.card_id),
                ("checklistId", &target.checklist_id),
            ] {
                if let Some(value) = value {
                    query.append_pair(key, value);
                }
            }
            if query_token {
                query.append_pair("authToken", token.expose_secret());
            }
        }
        let mut request = self
            .http
            .post(url)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/zip")
            .header(CONTENT_LENGTH, length)
            .body(reqwest::Body::wrap_stream(ReaderStream::new(reader)));
        if !query_token {
            request = request.bearer_auth(token.expose_secret());
        }
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        let body = self
            .execute_success_request(request)
            .await
            .map_err(|error| match error {
                ClientError::Transport(source) => ClientError::Transport(source.without_url()),
                ClientError::ResponseBody {
                    status,
                    retry_after_seconds,
                    source,
                } => ClientError::ResponseBody {
                    status,
                    retry_after_seconds,
                    source: source.without_url(),
                },
                other => other,
            })?;
        let response: ZipImportResponse = decode_success(&body, "ZIP import", true)?;
        if !response.ok {
            return Err(ClientError::Protocol {
                message: "ZIP import did not return ok: true".into(),
                diagnostic: None,
                success_status_received: true,
            });
        }
        Ok(response)
    }
}
