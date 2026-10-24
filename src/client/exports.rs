//! Native export transport. Complete artifacts are staged before validation/delivery.
mod documents;
mod dynamic_fields;
mod external_documents;
mod scrum_transfer;
mod validation;

use super::{
    ClientError, WekanClient,
    transport::{read_limited_body, require_success_status},
};
use reqwest::{
    StatusCode,
    header::{CONTENT_TYPE, RETRY_AFTER},
};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    time::Duration,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Csv,
    Tsv,
    Xlsx,
    Pdf,
    Zip,
    Ics,
    Wekan,
    Excel,
    Scsv,
    Kanboard,
    Deck,
    Openproject,
    Github,
    Gitlab,
    Gitea,
    Forgejo,
    Trello,
    Jira,
    Asana,
    Zenkit,
    Markdown,
    Leo,
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
    Businessmap,
    Superproductivity,
    Taiga,
    Vikunja,
    Wrike,
    Wrikeworkflow,
    Teamwork,
    Quire,
    Notion,
    Opml,
    Orgmode,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum ArtifactKind {
    Json,
    Csv,
    Text,
    Ics,
    Ndjson,
    Workbook,
    Pdf,
    Zip,
}
impl ExportFormat {
    pub const ALL: [Self; 50] = [
        Self::Json,
        Self::Csv,
        Self::Tsv,
        Self::Xlsx,
        Self::Pdf,
        Self::Zip,
        Self::Ics,
        Self::Wekan,
        Self::Excel,
        Self::Scsv,
        Self::Kanboard,
        Self::Deck,
        Self::Openproject,
        Self::Github,
        Self::Gitlab,
        Self::Gitea,
        Self::Forgejo,
        Self::Trello,
        Self::Jira,
        Self::Asana,
        Self::Zenkit,
        Self::Markdown,
        Self::Leo,
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
        Self::Businessmap,
        Self::Superproductivity,
        Self::Taiga,
        Self::Vikunja,
        Self::Wrike,
        Self::Wrikeworkflow,
        Self::Teamwork,
        Self::Quire,
        Self::Notion,
        Self::Opml,
        Self::Orgmode,
    ];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Xlsx => "xlsx",
            Self::Pdf => "pdf",
            Self::Zip => "zip",
            Self::Ics => "ics",
            Self::Wekan => "wekan",
            Self::Excel => "excel",
            Self::Scsv => "scsv",
            Self::Kanboard => "kanboard",
            Self::Deck => "deck",
            Self::Openproject => "openproject",
            Self::Github => "github",
            Self::Gitlab => "gitlab",
            Self::Gitea => "gitea",
            Self::Forgejo => "forgejo",
            Self::Trello => "trello",
            Self::Jira => "jira",
            Self::Asana => "asana",
            Self::Zenkit => "zenkit",
            Self::Markdown => "markdown",
            Self::Leo => "leo",
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
            Self::Businessmap => "businessmap",
            Self::Superproductivity => "superproductivity",
            Self::Taiga => "taiga",
            Self::Vikunja => "vikunja",
            Self::Wrike => "wrike",
            Self::Wrikeworkflow => "wrikeworkflow",
            Self::Teamwork => "teamwork",
            Self::Quire => "quire",
            Self::Notion => "notion",
            Self::Opml => "opml",
            Self::Orgmode => "orgmode",
        }
    }
    pub(super) fn suffix(self) -> &'static [&'static str] {
        match self {
            Self::Json => &["export"],
            Self::Csv => &["export", "csv"],
            Self::Tsv => &["export", "csv"],
            Self::Xlsx => &["exportExcel"],
            Self::Pdf => &["exportPDF"],
            Self::Zip => &["exportZip"],
            Self::Ics => &["calendar.ics"],
            Self::Wekan => &["export"],
            Self::Excel => &["exportExcel"],
            Self::Scsv => &["export", "csv"],
            Self::Kanboard => &["export", "kanboard"],
            Self::Deck => &["export", "deck"],
            Self::Openproject => &["export", "openproject"],
            Self::Github => &["export", "github"],
            Self::Gitlab => &["export", "gitlab"],
            Self::Gitea => &["export", "gitea"],
            Self::Forgejo => &["export", "forgejo"],
            Self::Trello => &["export", "trello"],
            Self::Jira => &["export", "jira"],
            Self::Asana => &["export", "asana"],
            Self::Zenkit => &["export", "zenkit"],
            Self::Markdown => &["export", "markdown"],
            Self::Leo => &["export", "leo"],
            Self::Todotxt => &["export", "todotxt"],
            Self::Taskwarrior => &["export", "taskwarrior"],
            Self::Focalboard => &["export", "focalboard"],
            Self::Todoist => &["export", "todoist"],
            Self::Planner => &["export", "planner"],
            Self::Meistertask => &["export", "meistertask"],
            Self::Obsidian => &["export", "obsidian"],
            Self::Linear => &["export", "linear"],
            Self::Ticktick => &["export", "ticktick"],
            Self::Clickup => &["export", "clickup"],
            Self::Nullboard => &["export", "nullboard"],
            Self::Kanri => &["export", "kanri"],
            Self::Pivotal => &["export", "pivotal"],
            Self::Redmine => &["export", "redmine"],
            Self::Tasksorg => &["export", "tasksorg"],
            Self::Monday => &["export", "monday"],
            Self::Businessmap => &["export", "businessmap"],
            Self::Superproductivity => &["export", "superproductivity"],
            Self::Taiga => &["export", "taiga"],
            Self::Vikunja => &["export", "vikunja"],
            Self::Wrike => &["export", "wrike"],
            Self::Wrikeworkflow => &["export", "wrikeworkflow"],
            Self::Teamwork => &["export", "teamwork"],
            Self::Quire => &["export", "quire"],
            Self::Notion => &["export", "notion"],
            Self::Opml => &["export", "opml"],
            Self::Orgmode => &["export", "orgmode"],
        }
    }
    fn kind(self) -> ArtifactKind {
        match self {
            Self::Json => ArtifactKind::Json,
            Self::Csv => ArtifactKind::Csv,
            Self::Tsv => ArtifactKind::Csv,
            Self::Xlsx => ArtifactKind::Workbook,
            Self::Pdf => ArtifactKind::Pdf,
            Self::Zip => ArtifactKind::Zip,
            Self::Ics => ArtifactKind::Ics,
            Self::Wekan => ArtifactKind::Json,
            Self::Excel => ArtifactKind::Workbook,
            Self::Scsv => ArtifactKind::Csv,
            Self::Kanboard => ArtifactKind::Json,
            Self::Deck => ArtifactKind::Json,
            Self::Openproject => ArtifactKind::Json,
            Self::Github => ArtifactKind::Json,
            Self::Gitlab => ArtifactKind::Json,
            Self::Gitea => ArtifactKind::Json,
            Self::Forgejo => ArtifactKind::Json,
            Self::Trello => ArtifactKind::Json,
            Self::Jira => ArtifactKind::Json,
            Self::Asana => ArtifactKind::Json,
            Self::Zenkit => ArtifactKind::Json,
            Self::Markdown => ArtifactKind::Text,
            Self::Leo => ArtifactKind::Text,
            Self::Todotxt => ArtifactKind::Text,
            Self::Taskwarrior => ArtifactKind::Json,
            Self::Focalboard => ArtifactKind::Ndjson,
            Self::Todoist => ArtifactKind::Csv,
            Self::Planner => ArtifactKind::Workbook,
            Self::Meistertask => ArtifactKind::Csv,
            Self::Obsidian => ArtifactKind::Text,
            Self::Linear => ArtifactKind::Csv,
            Self::Ticktick => ArtifactKind::Csv,
            Self::Clickup => ArtifactKind::Csv,
            Self::Nullboard => ArtifactKind::Json,
            Self::Kanri => ArtifactKind::Json,
            Self::Pivotal => ArtifactKind::Csv,
            Self::Redmine => ArtifactKind::Csv,
            Self::Tasksorg => ArtifactKind::Json,
            Self::Monday => ArtifactKind::Workbook,
            Self::Businessmap => ArtifactKind::Workbook,
            Self::Superproductivity => ArtifactKind::Json,
            Self::Taiga => ArtifactKind::Json,
            Self::Vikunja => ArtifactKind::Zip,
            Self::Wrike => ArtifactKind::Workbook,
            Self::Wrikeworkflow => ArtifactKind::Json,
            Self::Teamwork => ArtifactKind::Workbook,
            Self::Quire => ArtifactKind::Csv,
            Self::Notion => ArtifactKind::Csv,
            Self::Opml => ArtifactKind::Text,
            Self::Orgmode => ArtifactKind::Text,
        }
    }
    fn media_type_matches(self, value: &str) -> bool {
        match self {
            Self::Markdown => value == "text/markdown",
            Self::Obsidian => value == "text/markdown",
            Self::Leo => value == "application/xml",
            Self::Opml => value == "text/x-opml",
            Self::Orgmode => value == "text/x-org",
            Self::Todotxt => value == "text/plain",
            Self::Focalboard => value == "application/x-ndjson",
            Self::Ics => value == "text/calendar",
            format => match format.kind() {
                ArtifactKind::Csv => matches!(value, "text/csv" | "text/tsv"),
                ArtifactKind::Workbook => {
                    value == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                }
                ArtifactKind::Pdf => value == "application/pdf",
                ArtifactKind::Zip => value == "application/zip",
                ArtifactKind::Json => value == "application/json",
                _ => false,
            },
        }
    }
}
impl std::str::FromStr for ExportFormat {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|format| format.as_str() == value)
            .ok_or_else(|| "unsupported export format".into())
    }
}
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ExportTarget {
    Chart {
        board_id: String,
        chart_key: String,
    },
    AllBoards,
    Board {
        board_id: String,
    },
    Card {
        board_id: String,
        list_id: String,
        card_id: String,
    },
    Attachment {
        board_id: String,
        attachment_id: String,
    },
}
#[derive(Debug, Default)]
pub struct ExportRequest {
    pub board_ids: Vec<String>,
    pub chart: Option<ChartExportOptions>,
    pub fields: Vec<String>,
    pub swimlane_id: Option<String>,
    pub list_id: Option<String>,
    pub card_id: Option<String>,
    pub checklist_id: Option<String>,
    pub attachments: Option<bool>,
    pub delimiter: Option<String>,
    pub language: Option<String>,
    pub date_format: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartExportOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_days: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_field: Option<String>,
}

pub struct ExportOptions<'a> {
    pub format: ExportFormat,
    pub request: ExportRequest,
    pub token: Option<&'a SecretString>,
    pub query_token: bool,
    pub timeout: Option<Duration>,
}
#[derive(Debug, Eq, PartialEq)]
pub struct ExportMetadata {
    pub bytes_written: u64,
    pub http_status: u16,
    pub content_type: Option<String>,
}
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("export staging or validation failed")]
    Io(#[from] std::io::Error),
}
pub(super) fn protocol(message: &str) -> ClientError {
    ClientError::Protocol {
        diagnostic: None,
        message: message.into(),
        success_status_received: true,
    }
}

impl WekanClient {
    pub async fn export_board_chart(
        &self,
        board: &str,
        chart: &str,
        options: ExportOptions<'_>,
        file: &mut File,
    ) -> Result<ExportMetadata, ExportError> {
        let endpoint = if options.format == ExportFormat::Pdf {
            "exportPDF"
        } else {
            "exportExcel"
        };
        self.export_api_path(&["boards", board, "charts", chart, endpoint], options, file)
            .await
    }

    pub async fn export_all_boards(
        &self,
        options: ExportOptions<'_>,
        file: &mut File,
    ) -> Result<ExportMetadata, ExportError> {
        let native_format = options.format.as_str();
        let options = ExportOptions {
            format: if options.format == ExportFormat::Excel {
                ExportFormat::Xlsx
            } else {
                ExportFormat::Zip
            },
            ..options
        };
        self.export_api_path(&["export-all-boards", native_format], options, file)
            .await
    }

    pub(super) async fn export_resource(
        &self,
        segments: &[&str],
        options: ExportOptions<'_>,
        file: &mut File,
        attachment: bool,
    ) -> Result<ExportMetadata, ExportError> {
        let mut url = self
            .server()
            .join("api/boards/")
            .map_err(|_| protocol("could not build export endpoint"))?;
        url.path_segments_mut()
            .map_err(|_| protocol("could not build export endpoint"))?
            .pop_if_empty()
            .extend(segments);
        self.export_url(url, options, file, attachment).await
    }
    async fn export_api_path(
        &self,
        segments: &[&str],
        options: ExportOptions<'_>,
        file: &mut File,
    ) -> Result<ExportMetadata, ExportError> {
        let mut url = self
            .server()
            .join("api/")
            .map_err(|_| protocol("could not build export endpoint"))?;
        url.path_segments_mut()
            .map_err(|_| protocol("could not build export endpoint"))?
            .pop_if_empty()
            .extend(segments);
        self.export_url(url, options, file, false).await
    }
    async fn export_url(
        &self,
        mut url: url::Url,
        options: ExportOptions<'_>,
        file: &mut File,
        attachment: bool,
    ) -> Result<ExportMetadata, ExportError> {
        {
            let mut pairs = url.query_pairs_mut();
            let request = &options.request;
            if !request.board_ids.is_empty() {
                pairs.append_pair("boardIds", &request.board_ids.join(","));
            }
            if let Some(chart) = &request.chart {
                // All keys are statically typed above, never arbitrary query properties.
                let encoded = serde_json::to_value(chart).expect("chart options serialize");
                for (key, value) in encoded.as_object().expect("chart options are an object") {
                    pairs.append_pair(
                        key,
                        &value
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_string()),
                    );
                }
            }
            if !request.fields.is_empty() {
                pairs.append_pair("fields", &request.fields.join(","));
            }
            for (key, value) in [
                ("swimlaneId", &request.swimlane_id),
                ("listId", &request.list_id),
                ("cardId", &request.card_id),
                ("checklistId", &request.checklist_id),
            ] {
                if let Some(value) = value {
                    pairs.append_pair(key, value);
                }
            }
            if let Some(value) = request.attachments {
                pairs.append_pair("attachments", if value { "true" } else { "false" });
            }
            for (key, value) in [
                ("delimiter", &request.delimiter),
                ("lang", &request.language),
                ("dateFormat", &request.date_format),
                ("tz", &request.timezone),
            ] {
                if let Some(value) = value {
                    pairs.append_pair(key, value);
                }
            }
            if options.format == ExportFormat::Scsv {
                pairs.append_pair("delimiter", ";");
            }
            if options.format == ExportFormat::Tsv {
                pairs.append_pair("delimiter", "\t");
            }
            if options.query_token {
                if let Some(token) = options.token {
                    pairs.append_pair("authToken", token.expose_secret());
                }
            }
        }
        let mut request = self.http.get(url);
        if !options.query_token {
            if let Some(token) = options.token {
                request = request.bearer_auth(token.expose_secret());
            }
        }
        if let Some(timeout) = options.timeout {
            request = request.timeout(timeout);
        }
        let mut response = request
            .send()
            .await
            .map_err(|e| ClientError::Transport(e.without_url()))?;
        let status = response.status();
        if status.is_redirection() {
            return Err(ClientError::UnexpectedRedirect { status }.into());
        }
        let retry = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok());
        if status != StatusCode::OK {
            let body = read_limited_body(response, retry, 1024 * 1024).await?;
            require_success_status(status, body, retry)?;
            unreachable!("non-200 status must fail");
        }
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let media = content_type
            .as_deref()
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if !options.format.media_type_matches(&media) {
            // These exact pinned routes can send an HTTP-200 plain-text error,
            // including responses with no Content-Type. Never deliver it as a file.
            let body = read_limited_body(response, retry, 1024 * 1024).await?;
            if matches!(options.format, ExportFormat::Xlsx | ExportFormat::Pdf)
                && !attachment
                && (media.is_empty() || media == "text/plain")
            {
                let text = String::from_utf8_lossy(&body);
                return Err(ClientError::EmbeddedProtocol {
                    http_status: status,
                    server_error: None,
                    server_reason: None,
                    server_message: Some(text.chars().take(1024).collect()),
                    server_error_type: None,
                    server_is_client_safe: None,
                }
                .into());
            }
            return Err(protocol("the export response has an unexpected Content-Type").into());
        }
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        let mut bytes_written = 0u64;
        while let Some(chunk) =
            response
                .chunk()
                .await
                .map_err(|source| ClientError::ResponseBody {
                    status,
                    retry_after_seconds: retry,
                    source: source.without_url(),
                })?
        {
            file.write_all(&chunk)?;
            bytes_written = bytes_written
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| protocol("export byte count overflow"))?;
        }
        file.seek(SeekFrom::Start(0))?;
        match options.format.kind() {
            ArtifactKind::Csv | ArtifactKind::Text => {
                validation::validate_text(file)?;
            }
            ArtifactKind::Ics => {
                validation::validate_calendar(file)?;
            }
            ArtifactKind::Ndjson => {
                validation::validate_focalboard(file)?;
            }
            ArtifactKind::Pdf | ArtifactKind::Workbook | ArtifactKind::Zip => {
                let mut prefix = [0; 5];
                let count = file.read(&mut prefix)?;
                let valid = if options.format.kind() == ArtifactKind::Pdf {
                    count == 5 && &prefix == b"%PDF-"
                } else {
                    count >= 4 && (&prefix[..4] == b"PK\x03\x04" || &prefix[..4] == b"PK\x05\x06")
                };
                if !valid {
                    return Err(
                        protocol("the export response has an invalid file signature").into(),
                    );
                }
            }
            ArtifactKind::Json => validation::validate(file, options.format, attachment)?,
        }
        file.seek(SeekFrom::Start(0))?;
        Ok(ExportMetadata {
            bytes_written,
            http_status: status.as_u16(),
            content_type,
        })
    }
}
