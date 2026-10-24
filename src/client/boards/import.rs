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
    Excel,
    Jira,
    Kanboard,
    Deck,
    Openproject,
    Github,
    Gitlab,
    Gitea,
    Forgejo,
    Asana,
    Zenkit,
    Markdown,
    Leo,
    Opml,
    Orgmode,
    Todotxt,
    Taskwarrior,
    Focalboard,
    Todoist,
    Planner,
    Meistertask,
    Obsidian,
    Linear,
    Ticktick,
    Clickup,
    Nullboard,
    Kanri,
    Pivotal,
    Redmine,
    Tasksorg,
    Monday,
    Superproductivity,
    Taiga,
    Vikunja,
    Wrike,
    Teamwork,
    Businessmap,
    Quire,
    Notion,
    Plane,
    Planka,
    Mstodo,
    Kanbanflow,
    Basecamp,
    Googletasks,
    Googlekeep,
}

impl ImportSource {
    pub const ALL: [Self; 49] = [
        Self::Wekan,
        Self::Trello,
        Self::Csv,
        Self::Excel,
        Self::Jira,
        Self::Kanboard,
        Self::Deck,
        Self::Openproject,
        Self::Github,
        Self::Gitlab,
        Self::Gitea,
        Self::Forgejo,
        Self::Asana,
        Self::Zenkit,
        Self::Markdown,
        Self::Leo,
        Self::Opml,
        Self::Orgmode,
        Self::Todotxt,
        Self::Taskwarrior,
        Self::Focalboard,
        Self::Todoist,
        Self::Planner,
        Self::Meistertask,
        Self::Obsidian,
        Self::Linear,
        Self::Ticktick,
        Self::Clickup,
        Self::Nullboard,
        Self::Kanri,
        Self::Pivotal,
        Self::Redmine,
        Self::Tasksorg,
        Self::Monday,
        Self::Superproductivity,
        Self::Taiga,
        Self::Vikunja,
        Self::Wrike,
        Self::Teamwork,
        Self::Businessmap,
        Self::Quire,
        Self::Notion,
        Self::Plane,
        Self::Planka,
        Self::Mstodo,
        Self::Kanbanflow,
        Self::Basecamp,
        Self::Googletasks,
        Self::Googlekeep,
    ];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wekan => "wekan",
            Self::Trello => "trello",
            Self::Csv => "csv",
            Self::Excel => "excel",
            Self::Jira => "jira",
            Self::Kanboard => "kanboard",
            Self::Deck => "deck",
            Self::Openproject => "openproject",
            Self::Github => "github",
            Self::Gitlab => "gitlab",
            Self::Gitea => "gitea",
            Self::Forgejo => "forgejo",
            Self::Asana => "asana",
            Self::Zenkit => "zenkit",
            Self::Markdown => "markdown",
            Self::Leo => "leo",
            Self::Opml => "opml",
            Self::Orgmode => "orgmode",
            Self::Todotxt => "todotxt",
            Self::Taskwarrior => "taskwarrior",
            Self::Focalboard => "focalboard",
            Self::Todoist => "todoist",
            Self::Planner => "planner",
            Self::Meistertask => "meistertask",
            Self::Obsidian => "obsidian",
            Self::Linear => "linear",
            Self::Ticktick => "ticktick",
            Self::Clickup => "clickup",
            Self::Nullboard => "nullboard",
            Self::Kanri => "kanri",
            Self::Pivotal => "pivotal",
            Self::Redmine => "redmine",
            Self::Tasksorg => "tasksorg",
            Self::Monday => "monday",
            Self::Superproductivity => "superproductivity",
            Self::Taiga => "taiga",
            Self::Vikunja => "vikunja",
            Self::Wrike => "wrike",
            Self::Teamwork => "teamwork",
            Self::Businessmap => "businessmap",
            Self::Quire => "quire",
            Self::Notion => "notion",
            Self::Plane => "plane",
            Self::Planka => "planka",
            Self::Mstodo => "mstodo",
            Self::Kanbanflow => "kanbanflow",
            Self::Basecamp => "basecamp",
            Self::Googletasks => "googletasks",
            Self::Googlekeep => "googlekeep",
        }
    }
    pub fn is_text(self) -> bool {
        matches!(
            self,
            Self::Markdown
                | Self::Leo
                | Self::Opml
                | Self::Orgmode
                | Self::Todotxt
                | Self::Taskwarrior
                | Self::Focalboard
                | Self::Todoist
                | Self::Meistertask
                | Self::Obsidian
                | Self::Linear
                | Self::Ticktick
                | Self::Clickup
                | Self::Nullboard
                | Self::Pivotal
                | Self::Redmine
                | Self::Superproductivity
                | Self::Vikunja
                | Self::Quire
                | Self::Notion
                | Self::Plane
        )
    }
    pub fn is_workbook(self) -> bool {
        matches!(
            self,
            Self::Excel
                | Self::Planner
                | Self::Monday
                | Self::Wrike
                | Self::Teamwork
                | Self::Businessmap
        )
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
    #[serde(flatten)]
    pub options: BoardImportOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members_mapping: Option<BTreeMap<String, String>>,
}

impl BoardImportRequest {
    pub fn excel(bytes: &[u8], members_mapping: Option<BTreeMap<String, String>>) -> Self {
        Self {
            board: serde_json::json!({"excelBase64": STANDARD.encode(bytes)}),
            members_mapping,
            options: BoardImportOptions::default(),
        }
    }

    pub fn xlsx(bytes: &[u8], members_mapping: Option<BTreeMap<String, String>>) -> Self {
        Self {
            board: serde_json::json!({"xlsxBase64": STANDARD.encode(bytes)}),
            members_mapping,
            options: BoardImportOptions::default(),
        }
    }

    pub fn zip(bytes: &[u8], members_mapping: Option<BTreeMap<String, String>>) -> Self {
        Self {
            board: serde_json::json!({"zipBase64": STANDARD.encode(bytes)}),
            members_mapping,
            options: BoardImportOptions::default(),
        }
    }
}

/// Native per-import options from v12.57's importBoard and REST dispatch.
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardImportOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csv_mapping: Option<CsvImportMapping>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachment_auth: Option<ImportAttachmentAuth>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CsvImportMapping {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<BTreeMap<String, u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_field_columns: Option<Vec<u16>>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImportAttachmentAuth {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<crate::client::admin::StorageSecret>,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NewBoardZipImportResponse {
    pub ok: bool,
    pub board_id: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NewBoardZipImportTarget {
    pub fields: Vec<String>,
    pub members_mode: Option<String>,
}

enum ZipDestination<'a> {
    Existing(&'a ZipImportTarget),
    New(&'a NewBoardZipImportTarget),
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
        let body = self
            .import_zip_body(
                ZipDestination::Existing(target),
                reader,
                length,
                token,
                query_token,
                timeout,
            )
            .await?;
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

    pub async fn import_zip_new_board(
        &self,
        target: &NewBoardZipImportTarget,
        reader: impl AsyncRead + Send + Unpin + 'static,
        length: u64,
        token: &SecretString,
        query_token: bool,
        timeout: Option<Duration>,
    ) -> Result<NewBoardZipImportResponse, ClientError> {
        let body = self
            .import_zip_body(
                ZipDestination::New(target),
                reader,
                length,
                token,
                query_token,
                timeout,
            )
            .await?;
        let response: NewBoardZipImportResponse =
            decode_success(&body, "ZIP new-board import", true)?;
        if !response.ok {
            return Err(ClientError::Protocol {
                message: "ZIP import did not return ok: true".into(),
                diagnostic: None,
                success_status_received: true,
            });
        }
        require_non_empty(&response.board_id, "ZIP new-board import", "board id")?;
        Ok(response)
    }

    async fn import_zip_body(
        &self,
        target: ZipDestination<'_>,
        reader: impl AsyncRead + Send + Unpin + 'static,
        length: u64,
        token: &SecretString,
        query_token: bool,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>, ClientError> {
        let mut url = self.boards_api_endpoint("api/import/zip", "ZIP import")?;
        {
            let mut query = url.query_pairs_mut();
            let fields = match target {
                ZipDestination::Existing(target) => {
                    query.append_pair("boardId", &target.board_id);
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
                    &target.fields
                }
                ZipDestination::New(target) => {
                    query.append_pair("newBoard", "1");
                    if let Some(mode) = &target.members_mode {
                        query.append_pair("membersMode", mode);
                    }
                    &target.fields
                }
            };
            if !fields.is_empty() {
                query.append_pair("fields", &fields.join(","));
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
        self.execute_success_request(request)
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
            })
    }
}
