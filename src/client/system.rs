use std::collections::BTreeMap;

use reqwest::header::ACCEPT;
use serde::{Deserialize, Serialize};

use super::{ClientError, WekanClient, transport::decode_success};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct SchemaUpgradeState {
    pub running: bool,
    pub product: String,
    pub app_version: String,
    #[serde(deserialize_with = "nullable_date")]
    pub started_at: Option<String>,
    #[serde(deserialize_with = "nullable_date")]
    pub finished_at: Option<String>,
    pub current_step: String,
    pub gated: bool,
    pub steps: BTreeMap<String, SchemaUpgradeStepState>,
    #[serde(deserialize_with = "required_nullable")]
    pub last_check: Option<SchemaUpgradeLastCheck>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaUpgradeStepState {
    pub status: SchemaUpgradeStepStatus,
    pub fixed: i32,
    pub unresolved: i32,
    #[serde(default, deserialize_with = "present")]
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaUpgradeStepStatus {
    Pending,
    Checking,
    Running,
    Done,
    Skipped,
    Error,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaUpgradeLastCheck {
    pub version: String,
    #[serde(deserialize_with = "date")]
    pub at: String,
}

fn required_nullable<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}

fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

fn validate_date<E: serde::de::Error>(value: &str) -> Result<(), E> {
    super::datetime::validate_response_date(value)
        .map_err(|()| E::custom("invalid schema-upgrade date-time"))
}

fn date<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let value = String::deserialize(d)?;
    validate_date::<D::Error>(&value)?;
    Ok(value)
}

fn nullable_date<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    let value = Option::<String>::deserialize(d)?;
    if let Some(value) = &value {
        validate_date::<D::Error>(value)?;
    }
    Ok(value)
}

impl WekanClient {
    pub async fn schema_upgrade_status(&self) -> Result<SchemaUpgradeState, ClientError> {
        let mut endpoint =
            self.server()
                .join("schema-upgrade-status")
                .map_err(|_| ClientError::Protocol {
                    diagnostic: None,
                    message: "could not build the schema-upgrade status endpoint".into(),
                    success_status_received: false,
                })?;
        endpoint.query_pairs_mut().append_pair("json", "1");
        let body = self
            .execute_success_request(self.http.get(endpoint).header(ACCEPT, "application/json"))
            .await?;
        decode_success(&body, "schema-upgrade status", true)
    }
}
