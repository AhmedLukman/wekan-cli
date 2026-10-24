//! Stored v12.57 account channels. Response secrets become presence flags at decode.
use serde::{Deserialize, Deserializer, Serialize};

fn date<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let value = String::deserialize(d)?;
    crate::client::datetime::validate_response_date(&value)
        .map_err(|()| serde::de::Error::custom("invalid account-channel timestamp"))?;
    Ok(value)
}
fn optional_date<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    date(d).map(Some)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SecretPresence(bool);
impl<'de> Deserialize<'de> for SecretPresence {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d).map(|value| Self(!value.is_empty()))
    }
}

/// server/passkeys.js stores the public key as base64url text.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct Passkey {
    pub id: String,
    pub public_key: String,
    pub counter: u64,
    pub transports: Vec<String>,
    pub name: String,
    #[serde(deserialize_with = "date")]
    pub created_at: String,
    #[serde(default, deserialize_with = "optional_date")]
    pub last_used_at: Option<String>,
}

/// Public SQRL identity keys and options, from server/sqrl.js.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct SqrlIdentity {
    pub idk: String,
    pub suk: Option<String>,
    pub vuk: Option<String>,
    pub disabled: bool,
    #[serde(deserialize_with = "date")]
    pub added_at: String,
    pub sqrl_only: Option<bool>,
    pub hardlock: Option<bool>,
}

/// models/lib/webPush.js normalizes subscriptions before storing them.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct PushSubscription {
    #[serde(rename(deserialize = "endpoint", serialize = "endpoint_present"))]
    pub endpoint: SecretPresence,
    pub keys: PushKeys,
    #[serde(deserialize_with = "date")]
    pub created_at: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PushKeys {
    pub p256dh: String,
    #[serde(rename(deserialize = "auth", serialize = "auth_present"))]
    pub auth: SecretPresence,
}

/// models/lib/ntfy.js stores a topic URL and optional access token.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NtfySettings {
    pub enabled: bool,
    #[serde(rename(deserialize = "url", serialize = "url_present"))]
    pub url: SecretPresence,
    #[serde(rename(deserialize = "token", serialize = "token_present"))]
    pub token: SecretPresence,
}

/// server/lib/trayDelivery.js's bounded pending receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct PendingNotification {
    #[serde(rename(deserialize = "_id", serialize = "receipt_id"))]
    pub receipt_id: String,
    pub version: u64,
    pub user_id: String,
    pub activity_id: String,
}
