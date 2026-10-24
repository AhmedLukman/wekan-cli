use super::require_non_empty;
use crate::client::{ClientError, WekanClient, transport::decode_success};
use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Number;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BotRunUpdate {
    #[serde(
        default,
        deserialize_with = "crate::client::decoding::optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub status: Option<BotRunStatus>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub text: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub tool: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub question: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "crate::client::decoding::optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub options: Option<Vec<BotRunOption>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub tokens: Option<Option<Number>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub cost_usd: Option<Option<Number>>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BotRunOption {
    pub value: String,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub label: Option<Option<String>>,
}

fn present_nullable<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(deserializer).map(Some)
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BotRunStatus {
    Queued,
    Running,
    AwaitingInput,
    Review,
    Done,
    Failed,
    Stopped,
}
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BotRunReport {
    #[serde(rename(deserialize = "_id", serialize = "run_id"))]
    pub run_id: String,
    pub status: BotRunStatus,
}
impl WekanClient {
    pub async fn report_bot_run(
        &self,
        board: &str,
        card: &str,
        run: &str,
        update: &BotRunUpdate,
        token: &SecretString,
    ) -> Result<BotRunReport, ClientError> {
        for (value, field) in [(board, "board ID"), (card, "card ID"), (run, "run ID")] {
            require_non_empty(value, "bot run report", field)?;
        }
        let mut url = self
            .server()
            .join("api/boards/")
            .map_err(|_| super::protocol_error("could not build bot report endpoint".into()))?;
        url.path_segments_mut()
            .map_err(|_| super::protocol_error("could not build bot report endpoint".into()))?
            .pop_if_empty()
            .extend([board, "cards", card, "bot-runs", run]);
        let body = self
            .execute_success_request(
                self.http
                    .post(url)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(update),
            )
            .await?;
        let response: BotRunReport = decode_success(&body, "bot run report", true)?;
        if response.run_id != run {
            return Err(super::protocol_error(
                "the reported run ID did not match the requested run".into(),
            ));
        }
        Ok(response)
    }
}
