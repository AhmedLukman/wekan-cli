//! Native export transport. Complete artifacts are staged before validation/delivery.
mod documents;
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
}
impl ExportFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Xlsx => "xlsx",
            Self::Pdf => "pdf",
            Self::Zip => "zip",
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
        }
    }
    pub(super) fn suffix(self) -> &'static [&'static str] {
        match self {
            Self::Json => &["export"],
            Self::Csv | Self::Tsv => &["export", "csv"],
            Self::Xlsx => &["exportExcel"],
            Self::Pdf => &["exportPDF"],
            Self::Zip => &["exportZip"],
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
        }
    }
    fn media_type_matches(self, value: &str) -> bool {
        match self {
            Self::Csv | Self::Tsv => {
                matches!(value, "text/csv" | "text/tsv")
            }
            Self::Xlsx => {
                value == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            }
            Self::Pdf => value == "application/pdf",
            Self::Zip => value == "application/zip",
            _ => value == "application/json",
        }
    }
}
impl std::str::FromStr for ExportFormat {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "json" => Self::Json,
            "csv" => Self::Csv,
            "tsv" => Self::Tsv,
            "xlsx" => Self::Xlsx,
            "pdf" => Self::Pdf,
            "zip" => Self::Zip,
            "kanboard" => Self::Kanboard,
            "deck" => Self::Deck,
            "openproject" => Self::Openproject,
            "github" => Self::Github,
            "gitlab" => Self::Gitlab,
            "gitea" => Self::Gitea,
            "forgejo" => Self::Forgejo,
            "trello" => Self::Trello,
            "jira" => Self::Jira,
            "asana" => Self::Asana,
            "zenkit" => Self::Zenkit,
            _ => return Err("unsupported export format".into()),
        })
    }
}
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ExportTarget {
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
        {
            let mut pairs = url.query_pairs_mut();
            let request = &options.request;
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
        match options.format {
            ExportFormat::Csv | ExportFormat::Tsv => {}
            ExportFormat::Pdf | ExportFormat::Xlsx | ExportFormat::Zip => {
                let mut prefix = [0; 5];
                let count = file.read(&mut prefix)?;
                let valid = if options.format == ExportFormat::Pdf {
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
            format => validation::validate(file, format, attachment)?,
        }
        file.seek(SeekFrom::Start(0))?;
        Ok(ExportMetadata {
            bytes_written,
            http_status: status.as_u16(),
            content_type,
        })
    }
}
