use crate::client::{ClientError, WekanClient, transport::decode_success};
use reqwest::{Method, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Number;
use std::collections::BTreeMap;

pub const PROBLEM_STREAMS: [&str; 9] = [
    "security",
    "speed",
    "tests",
    "cpu",
    "ram",
    "disk",
    "database",
    "integrity",
    "mail",
];
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressProblem {
    pub kind: String,
    pub active: bool,
    pub message: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DetectedProblem {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub count: Option<Number>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NewProblemCount {
    pub stream: String,
    pub count: u64,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ProblemsOverview {
    pub any_in_progress: bool,
    pub in_progress: Vec<ProgressProblem>,
    pub any_problems: bool,
    pub problems: Vec<DetectedProblem>,
    pub new_problems: Vec<NewProblemCount>,
    pub streams: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventActor {
    pub kind: String,
    pub value: String,
    pub count: Number,
    pub at: String,
    pub family: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventLocation {
    pub country: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    pub latitude: Option<Number>,
    pub longitude: Option<Number>,
    pub via: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct ProblemEvent {
    #[serde(rename(deserialize = "_id", serialize = "event_id"))]
    pub event_id: String,
    pub stream: String,
    pub at: String,
    pub severity: Option<String>,
    pub category: Option<String>,
    pub bleed: Option<String>,
    pub action: Option<String>,
    pub source: Option<String>,
    pub cwe: Option<String>,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub ip: Option<String>,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub location: Option<EventLocation>,
    pub count: Option<Number>,
    pub first_at: Option<String>,
    pub actors: Option<BTreeMap<String, EventActor>>,
    pub actors_overflow: Option<Number>,
    pub detail: Option<String>,
    #[serde(rename(deserialize = "type", serialize = "event_type"))]
    pub event_type: Option<String>,
    pub db: Option<String>,
    pub kind: Option<String>,
    pub message: Option<String>,
    pub api: Option<String>,
    pub api_user_id: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProblemPage {
    pub stream: String,
    pub total: u64,
    pub limit: u64,
    pub skip: u64,
    pub rows: Vec<ProblemEvent>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProblemAcknowledgment {
    pub acknowledged: String,
}

impl WekanClient {
    pub async fn problems_overview(
        &self,
        token: &SecretString,
    ) -> Result<ProblemsOverview, ClientError> {
        self.admin_settings(
            Method::GET,
            "api/admin/problems",
            None,
            token,
            "admin problems overview",
        )
        .await
    }
    pub async fn problem_stream(
        &self,
        stream: &str,
        limit: u64,
        skip: u64,
        search: Option<&str>,
        token: &SecretString,
    ) -> Result<ProblemPage, ClientError> {
        let mut url =
            self.server()
                .join("api/admin/problems")
                .map_err(|_| ClientError::Protocol {
                    message: "could not construct problem stream endpoint".into(),
                    diagnostic: None,
                    success_status_received: false,
                })?;
        url.path_segments_mut()
            .expect("HTTP URL supports path segments")
            .push(stream);
        url.query_pairs_mut()
            .append_pair("limit", &limit.to_string())
            .append_pair("skip", &skip.to_string());
        if let Some(search) = search {
            url.query_pairs_mut().append_pair("search", search);
        }
        let body = self
            .execute_success_request_with_limit(
                self.http
                    .get(url)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
                64 * 1024 * 1024,
            )
            .await?;
        let page: ProblemPage = decode_success(&body, "admin problem stream", true)?;
        if page.stream != stream {
            return Err(ClientError::Protocol {
                message: "the returned problem stream did not match the requested stream".into(),
                diagnostic: None,
                success_status_received: true,
            });
        }
        Ok(page)
    }
    pub async fn acknowledge_problem_stream(
        &self,
        stream: &str,
        token: &SecretString,
    ) -> Result<ProblemAcknowledgment, ClientError> {
        let mut url =
            self.server()
                .join("api/admin/problems")
                .map_err(|_| ClientError::Protocol {
                    message: "could not construct problem acknowledgment endpoint".into(),
                    diagnostic: None,
                    success_status_received: false,
                })?;
        url.path_segments_mut()
            .expect("HTTP URL supports path segments")
            .extend([stream, "acknowledge"]);
        let body = self
            .execute_success_request(
                self.http
                    .post(url)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let response: ProblemAcknowledgment =
            decode_success(&body, "admin problem acknowledgment", true)?;
        if response.acknowledged != stream {
            return Err(ClientError::Protocol {
                message: "the acknowledged problem stream did not match the request".into(),
                diagnostic: None,
                success_status_received: true,
            });
        }
        Ok(response)
    }
}
