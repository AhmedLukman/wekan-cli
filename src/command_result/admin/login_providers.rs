use crate::client::admin::login_providers as wire;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ConfigSource<T> {
    pub source: String,
    pub value: Option<T>,
}

impl<T, U: From<T>> From<wire::ConfigSource<T>> for ConfigSource<U> {
    fn from(source: wire::ConfigSource<T>) -> Self {
        Self {
            source: source.source,
            value: source.value.map(Into::into),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BooleanConfigValue {
    Text(String),
    Boolean(bool),
}

impl From<wire::BooleanConfigValue> for BooleanConfigValue {
    fn from(value: wire::BooleanConfigValue) -> Self {
        match value {
            wire::BooleanConfigValue::Text(value) => Self::Text(value),
            wire::BooleanConfigValue::Boolean(value) => Self::Boolean(value),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SecretConfigSource {
    pub source: String,
    pub has_value: bool,
}

impl From<wire::SecretConfigSource> for SecretConfigSource {
    fn from(source: wire::SecretConfigSource) -> Self {
        Self {
            source: source.source,
            has_value: source.has_value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OauthProviderSource {
    pub enabled: ConfigSource<BooleanConfigValue>,
    pub id: ConfigSource<String>,
    pub secret: SecretConfigSource,
}

impl From<wire::OauthProviderSource> for OauthProviderSource {
    fn from(provider: wire::OauthProviderSource) -> Self {
        Self {
            enabled: provider.enabled.into(),
            id: provider.id.into(),
            secret: provider.secret.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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

impl From<wire::OauthProviderSources> for OauthProviderSources {
    fn from(providers: wire::OauthProviderSources) -> Self {
        Self {
            google: providers.google.into(),
            github: providers.github.into(),
            facebook: providers.facebook.into(),
            twitter: providers.twitter.into(),
            meteor_developer: providers.meteor_developer.into(),
            weibo: providers.weibo.into(),
            meetup: providers.meetup.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OauthProviderSettings {
    pub providers: OauthProviderSources,
    pub login_style: ConfigSource<String>,
    pub merge_existing_users: ConfigSource<BooleanConfigValue>,
    pub allowed_email_domains: ConfigSource<String>,
    pub passwordless: ConfigSource<BooleanConfigValue>,
}

impl From<wire::OauthProviderSettings> for OauthProviderSettings {
    fn from(settings: wire::OauthProviderSettings) -> Self {
        Self {
            providers: settings.providers.into(),
            login_style: settings.login_style.into(),
            merge_existing_users: settings.merge_existing_users.into(),
            allowed_email_domains: settings.allowed_email_domains.into(),
            passwordless: settings.passwordless.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PasswordlessSettings {
    pub passwordless: ConfigSource<BooleanConfigValue>,
}

impl From<wire::PasswordlessSettings> for PasswordlessSettings {
    fn from(settings: wire::PasswordlessSettings) -> Self {
        Self {
            passwordless: settings.passwordless.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn provider_results_preserve_keys_sources_nulls_and_boolean_representations() {
        let provider = json!({
            "enabled":{"source":"env","value":"false"},
            "id":{"source":"default"},
            "secret":{"source":"database","hasValue":true}
        });
        let providers: serde_json::Map<_, _> = [
            "google",
            "github",
            "facebook",
            "twitter",
            "meteor-developer",
            "weibo",
            "meetup",
        ]
        .into_iter()
        .map(|key| (key.to_owned(), provider.clone()))
        .collect();
        let response: wire::OauthProviderSettings = serde_json::from_value(json!({
            "providers":providers,
            "loginStyle":{"source":"database","value":"popup"},
            "mergeExistingUsers":{"source":"database","value":true},
            "allowedEmailDomains":{"source":"default","value":null},
            "passwordless":{"source":"env","value":"true"}
        }))
        .unwrap();
        let stable_provider = json!({
            "enabled":{"source":"env","value":"false"},
            "id":{"source":"default","value":null},
            "secret":{"source":"database","has_value":true}
        });
        let stable_providers: serde_json::Map<_, _> = [
            "google",
            "github",
            "facebook",
            "twitter",
            "meteor-developer",
            "weibo",
            "meetup",
        ]
        .into_iter()
        .map(|key| (key.to_owned(), stable_provider.clone()))
        .collect();
        assert_eq!(
            serde_json::to_value(OauthProviderSettings::from(response)).unwrap(),
            json!({
                "providers":stable_providers,
                "login_style":{"source":"database","value":"popup"},
                "merge_existing_users":{"source":"database","value":true},
                "allowed_email_domains":{"source":"default","value":null},
                "passwordless":{"source":"env","value":"true"}
            })
        );
        for value in [json!(false), json!("false"), json!(null)] {
            let response: wire::PasswordlessSettings = serde_json::from_value(json!({
                "passwordless":{"source":"default","value":value}
            }))
            .unwrap();
            assert_eq!(
                serde_json::to_value(PasswordlessSettings::from(response)).unwrap(),
                json!({"passwordless":{"source":"default","value":value}})
            );
        }
    }
}
