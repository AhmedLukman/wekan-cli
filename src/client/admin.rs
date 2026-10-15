//! Global administrator settings for Wekan v11.06.
pub mod attachment_settings;
pub mod people;
pub mod settings;

use super::{ClientError, WekanClient, transport::decode_success};
use reqwest::{
    Method,
    header::{ACCEPT, CONTENT_TYPE},
};
use secrecy::{ExposeSecret, SecretString};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, DeserializeOwned, MapAccess, Visitor},
};
use serde_json::{Number, Value};

pub const MAX_SETTINGS_BYTES: usize = 50 * 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

/// A typed settings update encoded once, before authentication or transport.
pub struct PreparedSettingsUpdate<T> {
    body: Vec<u8>,
    request: std::marker::PhantomData<fn() -> T>,
}

#[derive(Debug, thiserror::Error)]
#[error("serialized settings cannot be encoded within the 50 MiB limit; no request was sent")]
pub struct SettingsEncodeError;

impl<T: Serialize> PreparedSettingsUpdate<T> {
    pub fn new(request: &T) -> Result<Self, SettingsEncodeError> {
        Ok(Self {
            body: encode_bounded(request, MAX_SETTINGS_BYTES)?,
            request: std::marker::PhantomData,
        })
    }
}

pub(super) fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

/// Serde structs also accept positional arrays; API response objects must be maps.
pub(crate) struct Object<T>(pub T);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct ObjectVisitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
            type Value = Object<T>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
                T::deserialize(de::value::MapAccessDeserializer::new(map)).map(Object)
            }
        }
        d.deserialize_map(ObjectVisitor(std::marker::PhantomData))
    }
}
pub(super) fn present_object<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Object::<T>::deserialize(d).map(|v| Some(v.0))
}
pub(super) fn nullable_object<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<Object<T>>::deserialize(d).map(|v| v.map(|v| v.0))
}
pub(super) fn date<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    let value = String::deserialize(d)?;
    super::datetime::validate_response_date(&value)
        .map_err(|()| de::Error::custom("invalid settings date"))?;
    Ok(value)
}

macro_rules! numeric_setting {
    ($name:ident, $min:expr, $max:expr) => {
        #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(Number);
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let number = Number::deserialize(d)?;
                let value = number
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| de::Error::custom("expected a finite settings number"))?;
                let min: Option<f64> = $min;
                let max: Option<f64> = $max;
                if min.is_some_and(|m| value < m) || max.is_some_and(|m| value > m) {
                    return Err(de::Error::custom(
                        "settings number is outside the supported range",
                    ));
                }
                Ok(Self(number))
            }
        }
        impl std::str::FromStr for $name {
            type Err = &'static str;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                serde_json::from_str(value)
                    .map_err(|_| "expected a finite JSON number within the setting's range")
            }
        }
    };
}
numeric_setting!(SettingNumber, None, None);
numeric_setting!(NonNegativeNumber, Some(0.0), None);
numeric_setting!(BatchSize, Some(1.0), Some(100.0));
numeric_setting!(DelayMs, Some(100.0), Some(10000.0));
numeric_setting!(CpuThreshold, Some(10.0), Some(90.0));

/// A response secret must have been masked by Wekan before decoding succeeds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaskedSecret;
impl<'de> Deserialize<'de> for MaskedSecret {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if String::deserialize(d)?.is_empty() {
            Ok(Self)
        } else {
            Err(de::Error::custom("unmasked storage credential"))
        }
    }
}

#[derive(Clone, Debug)]
pub struct StorageSecret(pub(crate) SecretString);
impl<'de> Deserialize<'de> for StorageSecret {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d).map(|v| Self(SecretString::from(v)))
    }
}
impl Serialize for StorageSecret {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.0.expose_secret())
    }
}

/// Credential objects are opaque request secrets, never response field maps.
#[derive(Clone, Debug)]
pub struct GcsCredentials {
    value: SecretString,
    object: bool,
}
impl GcsCredentials {
    pub(crate) fn text(value: SecretString) -> Self {
        Self {
            value,
            object: false,
        }
    }
    pub(crate) fn redaction_secrets(&self) -> Vec<SecretString> {
        fn collect(value: &Value, secrets: &mut Vec<SecretString>) {
            match value {
                Value::String(s) => secrets.push(SecretString::from(s.clone())),
                Value::Object(m) => m.values().for_each(|v| collect(v, secrets)),
                Value::Array(a) => a.iter().for_each(|v| collect(v, secrets)),
                _ => {}
            }
        }
        let mut secrets = vec![self.value.clone()];
        if let Ok(value) = serde_json::from_str::<Value>(self.value.expose_secret()) {
            collect(&value, &mut secrets);
        }
        secrets
    }
}
impl<'de> Deserialize<'de> for GcsCredentials {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match Value::deserialize(d)? {
            Value::String(s) => Ok(Self::text(SecretString::from(s))),
            Value::Object(map) => Ok(Self {
                value: SecretString::from(Value::Object(map).to_string()),
                object: true,
            }),
            _ => Err(de::Error::custom(
                "expected credential text or a credential object",
            )),
        }
    }
}
impl Serialize for GcsCredentials {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.object {
            // Only validated JSON objects can construct this representation.
            serde_json::from_str::<Value>(self.value.expose_secret())
                .map_err(serde::ser::Error::custom)?
                .serialize(s)
        } else {
            s.serialize_str(self.value.expose_secret())
        }
    }
}

impl WekanClient {
    async fn admin_settings<T: DeserializeOwned>(
        &self,
        method: Method,
        route: &str,
        body: Option<Vec<u8>>,
        token: &SecretString,
        operation: &str,
    ) -> Result<T, ClientError> {
        let endpoint = self
            .server()
            .join(route)
            .map_err(|_| ClientError::Protocol {
                message: "could not construct settings endpoint; no request was sent".into(),
                diagnostic: None,
                success_status_received: false,
            })?;
        let mut request = self
            .http
            .request(method, endpoint)
            .header(ACCEPT, "application/json")
            .bearer_auth(token.expose_secret());
        if let Some(body) = body {
            request = request.header(CONTENT_TYPE, "application/json").body(body);
        }
        let body = self
            .execute_success_request_with_limit(request, MAX_RESPONSE_BYTES)
            .await?;
        decode_success::<Object<T>>(&body, operation, true).map(|v| v.0)
    }
    pub async fn global_settings(
        &self,
        token: &SecretString,
    ) -> Result<settings::GlobalSettingsDocument, ClientError> {
        self.admin_settings(
            Method::GET,
            "api/settings",
            None,
            token,
            "admin settings get",
        )
        .await
    }
    pub async fn update_global_settings(
        &self,
        request: PreparedSettingsUpdate<settings::UpdateGlobalSettings>,
        token: &SecretString,
    ) -> Result<settings::GlobalSettingsDocument, ClientError> {
        self.admin_settings(
            Method::PUT,
            "api/settings",
            Some(request.body),
            token,
            "admin settings update",
        )
        .await
    }
    pub async fn attachment_settings(
        &self,
        token: &SecretString,
    ) -> Result<attachment_settings::AttachmentSettingsDocument, ClientError> {
        self.admin_settings(
            Method::GET,
            "api/admin/attachment-settings",
            None,
            token,
            "admin attachment-settings get",
        )
        .await
    }
    pub async fn update_attachment_settings(
        &self,
        request: PreparedSettingsUpdate<attachment_settings::UpdateAttachmentSettings>,
        token: &SecretString,
    ) -> Result<attachment_settings::AttachmentSettingsDocument, ClientError> {
        self.admin_settings(
            Method::PUT,
            "api/admin/attachment-settings",
            Some(request.body),
            token,
            "admin attachment-settings update",
        )
        .await
    }
}

fn encode_bounded<T: Serialize>(request: &T, limit: usize) -> Result<Vec<u8>, SettingsEncodeError> {
    struct BoundedBody {
        bytes: Vec<u8>,
        limit: usize,
    }
    impl std::io::Write for BoundedBody {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
                return Err(std::io::Error::other("settings size limit"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut body = BoundedBody {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut body, request).map_err(|_| SettingsEncodeError)?;
    Ok(body.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn preparation_serializes_once_and_bounds_encoded_bytes_including_escapes() {
        struct Counted(Cell<usize>);
        impl Serialize for Counted {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.0.set(self.0.get() + 1);
                serializer.serialize_str("line\n")
            }
        }
        let request = Counted(Cell::new(0));
        let prepared = PreparedSettingsUpdate::new(&request).unwrap();
        assert_eq!(request.0.get(), 1);
        let expected = b"\"line\\n\"";
        assert_eq!(prepared.body, expected);
        assert_eq!(encode_bounded(&"line\n", expected.len()).unwrap(), expected);
        assert!(encode_bounded(&"line\n", expected.len() - 1).is_err());
    }
}
