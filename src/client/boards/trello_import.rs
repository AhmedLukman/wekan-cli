//! The application HTTP importer is distinct from the /api import engines.
use std::time::Duration;

use reqwest::{
    RequestBuilder,
    header::{ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, COOKIE},
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncRead;
use tokio_util::io::ReaderStream;

use super::import::BoardImportRequest;
use crate::client::{
    ClientError, WekanClient, imports::PreparedImportRequest, transport::decode_success,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrelloImportFormat {
    Json,
    Zip,
}

#[derive(Clone, Copy, Debug)]
pub enum TrelloImportAuth {
    Bearer,
    XAuthToken,
    MeteorCookie,
    WekanCookie,
}

/// Wekan echoes an unvalidated name on failure. Compound names are deliberately
/// rejected rather than admitting unmapped nested response objects.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TrelloImportBoardName {
    String(String),
    Number(serde_json::Number),
    Boolean(bool),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrelloImportBoardError {
    pub board: TrelloImportBoardName,
    pub error: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonResponse {
    #[serde(rename = "boardIds")]
    board_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ZipResponse {
    #[serde(rename = "boardIds")]
    board_ids: Vec<String>,
    errors: Vec<TrelloImportBoardError>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorResponse {
    #[serde(rename = "error")]
    _error: String,
}

#[derive(Debug)]
pub struct TrelloImportResponse {
    pub board_ids: Vec<String>,
    pub errors: Option<Vec<TrelloImportBoardError>>,
}

impl WekanClient {
    pub async fn import_trello_json(
        &self,
        body: PreparedImportRequest<BoardImportRequest>,
        token: &SecretString,
        auth: TrelloImportAuth,
        timeout: Option<Duration>,
    ) -> Result<TrelloImportResponse, ClientError> {
        let request = self
            .http
            .post(self.boards_api_endpoint("import-trello", "Trello import")?)
            .header(CONTENT_TYPE, "application/json")
            .body(body.into_body());
        self.send_trello_import(request, TrelloImportFormat::Json, token, auth, timeout)
            .await
    }

    pub async fn import_trello_zip(
        &self,
        reader: impl AsyncRead + Send + Unpin + 'static,
        length: u64,
        token: &SecretString,
        auth: TrelloImportAuth,
        members_mode: Option<&str>,
        timeout: Option<Duration>,
    ) -> Result<TrelloImportResponse, ClientError> {
        let mut endpoint = self.boards_api_endpoint("import-trello", "Trello import")?;
        if let Some(mode) = members_mode {
            endpoint.query_pairs_mut().append_pair("membersMode", mode);
        }
        let request = self
            .http
            .post(endpoint)
            .header(CONTENT_TYPE, "application/zip")
            .header(CONTENT_LENGTH, length)
            .body(reqwest::Body::wrap_stream(ReaderStream::new(reader)));
        self.send_trello_import(request, TrelloImportFormat::Zip, token, auth, timeout)
            .await
    }

    async fn send_trello_import(
        &self,
        request: RequestBuilder,
        format: TrelloImportFormat,
        token: &SecretString,
        auth: TrelloImportAuth,
        timeout: Option<Duration>,
    ) -> Result<TrelloImportResponse, ClientError> {
        let mut request = request.header(ACCEPT, "application/json");
        request = match auth {
            TrelloImportAuth::Bearer => request.bearer_auth(token.expose_secret()),
            TrelloImportAuth::XAuthToken => request.header("X-Auth-Token", token.expose_secret()),
            TrelloImportAuth::MeteorCookie | TrelloImportAuth::WekanCookie => {
                let name = if matches!(auth, TrelloImportAuth::MeteorCookie) {
                    "meteor_login_token"
                } else {
                    "wekan_login_token"
                };
                let encoded =
                    url::form_urlencoded::byte_serialize(token.expose_secret().as_bytes())
                        .collect::<String>()
                        .replace('+', "%20");
                request.header(COOKIE, format!("{name}={encoded}"))
            }
        };
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        let body = self
            .execute_success_request_with_error::<ClientError>(request, |_, body| {
                // Middleware may return HTML. JSON errors belong to this route's
                // stricter {error: string} contract; retain shared HTTP metadata.
                if let Ok(value) = serde_json::from_slice(body) {
                    let _: ErrorResponse =
                        crate::client::decoding::decode_value(value, "Trello import error", false)?;
                }
                Ok(None)
            })
            .await?;
        match format {
            TrelloImportFormat::Json => {
                let response: JsonResponse = decode_success(&body, "Trello JSON import", true)?;
                Ok(TrelloImportResponse {
                    board_ids: response.board_ids,
                    errors: None,
                })
            }
            TrelloImportFormat::Zip => {
                let response: ZipResponse = decode_success(&body, "Trello ZIP import", true)?;
                Ok(TrelloImportResponse {
                    board_ids: response.board_ids,
                    errors: Some(response.errors),
                })
            }
        }
    }
}
