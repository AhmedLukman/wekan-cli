use super::{PreparedSettingsUpdate, StorageSecret};
use crate::client::{ClientError, WekanClient};
use reqwest::{
    Method,
    header::{ACCEPT, CONTENT_TYPE},
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

pub const OAUTH_PROVIDER_KEYS: [&str; 7] = [
    "google",
    "github",
    "facebook",
    "twitter",
    "meteor-developer",
    "weibo",
    "meetup",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigSource<T> {
    pub source: String,
    pub value: Option<T>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BooleanConfigValue {
    Text(String),
    Boolean(bool),
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct SecretConfigSource {
    pub source: String,
    pub has_value: bool,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OauthProviderSource {
    pub enabled: ConfigSource<BooleanConfigValue>,
    pub id: ConfigSource<String>,
    pub secret: SecretConfigSource,
}
/// Every entry in v12.57's oauthProviderCatalog is present, even when unset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OauthProviderSources {
    pub google: OauthProviderSource,
    pub github: OauthProviderSource,
    pub facebook: OauthProviderSource,
    pub twitter: OauthProviderSource,
    #[serde(rename = "meteor-developer")]
    pub meteor_developer: OauthProviderSource,
    pub weibo: OauthProviderSource,
    pub meetup: OauthProviderSource,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct OauthProviderSettings {
    pub providers: OauthProviderSources,
    pub login_style: ConfigSource<String>,
    pub merge_existing_users: ConfigSource<BooleanConfigValue>,
    pub allowed_email_domains: ConfigSource<String>,
    pub passwordless: ConfigSource<BooleanConfigValue>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOauthProviderRequest {
    pub enabled: bool,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<StorageSecret>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_login_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_existing_users: Option<bool>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordlessSettings {
    pub passwordless: ConfigSource<BooleanConfigValue>,
}
#[derive(Serialize)]
struct PasswordlessRequest {
    enabled: bool,
}

impl WekanClient {
    pub async fn oauth_provider_settings(
        &self,
        token: &SecretString,
    ) -> Result<OauthProviderSettings, ClientError> {
        self.admin_settings(
            Method::GET,
            "api/admin/oauth-providers",
            None,
            token,
            "OAuth provider settings",
        )
        .await
    }
    pub async fn update_oauth_provider(
        &self,
        provider: &str,
        request: PreparedSettingsUpdate<UpdateOauthProviderRequest>,
        token: &SecretString,
    ) -> Result<OauthProviderSettings, ClientError> {
        let mut url = self
            .server()
            .join("api/admin/oauth-providers")
            .map_err(|_| ClientError::Protocol {
                message: "could not construct OAuth provider endpoint".into(),
                diagnostic: None,
                success_status_received: false,
            })?;
        url.path_segments_mut()
            .expect("HTTP URL supports path segments")
            .push(provider);
        let body = self
            .execute_success_request_with_limit(
                self.http
                    .put(url)
                    .header(ACCEPT, "application/json")
                    .header(CONTENT_TYPE, "application/json")
                    .bearer_auth(token.expose_secret())
                    .body(request.body),
                64 * 1024 * 1024,
            )
            .await?;
        crate::client::transport::decode_success::<super::Object<OauthProviderSettings>>(
            &body,
            "OAuth provider settings update",
            true,
        )
        .map(|value| value.0)
    }
    pub async fn update_passwordless_settings(
        &self,
        enabled: bool,
        token: &SecretString,
    ) -> Result<PasswordlessSettings, ClientError> {
        let body = serde_json::to_vec(&PasswordlessRequest { enabled })
            .expect("boolean request is serializable");
        self.admin_settings(
            Method::PUT,
            "api/admin/passwordless",
            Some(body),
            token,
            "passwordless settings update",
        )
        .await
    }
}
