//! Board automation rules for the pinned Wekan REST API.
use reqwest::{Method, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use super::{
    ClientError, WekanClient, datetime::validate_response_date, transport::decode_success,
};

mod payloads;
pub use payloads::{RuleAction, RuleActionInput, RuleNumber, RuleTrigger, RuleTriggerInput};

// Allow the 50 MiB input budget plus server defaults and response metadata.
const MAX_RULE_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuleDocument {
    #[serde(rename = "_id")]
    pub rule_id: String,
    pub title: String,
    #[serde(deserialize_with = "nullable_object")]
    pub trigger: Option<RuleTrigger>,
    #[serde(deserialize_with = "nullable_object")]
    pub action: Option<RuleAction>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RuleCreateResponse {
    #[serde(rename = "_id")]
    pub rule_id: String,
    pub trigger_id: String,
    pub action_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuleIdResponse {
    #[serde(rename = "_id")]
    pub rule_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateRuleRequest {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "present_string"
    )]
    pub title: Option<String>,
    #[serde(deserialize_with = "object")]
    pub trigger: RuleTriggerInput,
    #[serde(deserialize_with = "object")]
    pub action: RuleActionInput,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateRuleRequest {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "present_string"
    )]
    pub title: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "optional_object"
    )]
    pub trigger: Option<RuleTriggerInput>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "optional_object"
    )]
    pub action: Option<RuleActionInput>,
}

fn present_string<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}
fn present_nullable<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(d).map(Some)
}

// Named structs otherwise accept positional arrays through Serde's sequence visitor.
pub(crate) struct Object<T>(pub(crate) T);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Visitor<T> {
            type Value = Object<T>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                map: M,
            ) -> Result<Self::Value, M::Error> {
                T::deserialize(serde::de::value::MapAccessDeserializer::new(map)).map(Object)
            }
        }
        d.deserialize_map(Visitor(std::marker::PhantomData))
    }
}
fn object<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<T, D::Error> {
    Object::<T>::deserialize(d).map(|v| v.0)
}
fn nullable_object<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<Object<T>>::deserialize(d).map(|v| v.map(|v| v.0))
}
fn optional_object<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    object(d).map(Some)
}

impl WekanClient {
    pub async fn board_rules(
        &self,
        board: &str,
        token: &SecretString,
    ) -> Result<Vec<RuleDocument>, ClientError> {
        let body = self.rule_request(Method::GET, board, None, token).await?;
        let documents: Vec<Object<RuleDocument>> = decode_success(&body, "rule collection", true)?;
        let documents: Vec<_> = documents.into_iter().map(|v| v.0).collect();
        for d in &documents {
            validate_document(d, None)?;
        }
        Ok(documents)
    }
    pub async fn rule(
        &self,
        board: &str,
        rule: &str,
        token: &SecretString,
    ) -> Result<RuleDocument, ClientError> {
        let body = self
            .rule_request(Method::GET, board, Some(rule), token)
            .await?;
        let document: Object<RuleDocument> = decode_success(&body, "rule lookup", true)?;
        validate_document(&document.0, Some(rule))?;
        Ok(document.0)
    }
    pub async fn create_rule(
        &self,
        board: &str,
        request: &CreateRuleRequest,
        token: &SecretString,
    ) -> Result<RuleCreateResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.rule_endpoint(board, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: Object<RuleCreateResponse> = decode_success(&body, "rule creation", true)?;
        for id in [
            &response.0.rule_id,
            &response.0.trigger_id,
            &response.0.action_id,
        ] {
            validate_id(id, None)?;
        }
        Ok(response.0)
    }
    pub async fn update_rule(
        &self,
        board: &str,
        rule: &str,
        request: &UpdateRuleRequest,
        token: &SecretString,
    ) -> Result<RuleIdResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.rule_endpoint(board, Some(rule))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        decode_id(&body, rule)
    }
    pub async fn delete_rule(
        &self,
        board: &str,
        rule: &str,
        token: &SecretString,
    ) -> Result<RuleIdResponse, ClientError> {
        decode_id(
            &self
                .rule_request(Method::DELETE, board, Some(rule), token)
                .await?,
            rule,
        )
    }
    async fn rule_request(
        &self,
        method: Method,
        board: &str,
        rule: Option<&str>,
        token: &SecretString,
    ) -> Result<Vec<u8>, ClientError> {
        let is_read = method == Method::GET;
        let request = self
            .http
            .request(method, self.rule_endpoint(board, rule)?)
            .header(ACCEPT, "application/json")
            .bearer_auth(token.expose_secret());
        if is_read {
            self.execute_success_request_with_limit(request, MAX_RULE_RESPONSE_BYTES)
                .await
        } else {
            self.execute_success_request(request).await
        }
    }
    fn rule_endpoint(&self, board: &str, rule: Option<&str>) -> Result<url::Url, ClientError> {
        for id in [Some(board), rule].into_iter().flatten() {
            if super::trim_javascript_whitespace(id).is_empty() || matches!(id, "." | "..") {
                return Err(ClientError::Protocol {
                    diagnostic: None,
                    message:
                        "rule endpoint IDs must be nonblank and not dot-only; no request was sent"
                            .into(),
                    success_status_received: false,
                });
            }
        }
        let mut endpoint = self
            .server()
            .join("api/boards")
            .map_err(|_| protocol_error("could not build the rule endpoint"))?;
        let mut segments = endpoint
            .path_segments_mut()
            .map_err(|()| protocol_error("the Wekan URL cannot contain API paths"))?;
        segments.extend([board, "rules"]);
        if let Some(rule) = rule {
            segments.push(rule);
        }
        drop(segments);
        Ok(endpoint)
    }
}
fn decode_id(body: &[u8], expected: &str) -> Result<RuleIdResponse, ClientError> {
    let d: Object<RuleIdResponse> = decode_success(body, "rule acknowledgement", true)?;
    validate_id(&d.0.rule_id, Some(expected))?;
    Ok(d.0)
}
fn validate_id(id: &str, expected: Option<&str>) -> Result<(), ClientError> {
    if super::trim_javascript_whitespace(id).is_empty() {
        return Err(protocol_error(
            "the rule response contained an empty identifier",
        ));
    }
    if expected.is_some_and(|v| id != v) {
        return Err(protocol_error(
            "Wekan returned a different rule ID than requested",
        ));
    }
    Ok(())
}
fn validate_document(d: &RuleDocument, expected: Option<&str>) -> Result<(), ClientError> {
    validate_id(&d.rule_id, expected)?;
    if let Some(Some(date)) = d.trigger.as_ref().and_then(|t| t.last_run_at.as_ref()) {
        validate_response_date(date).map_err(|()| {
            protocol_error("the rule response contained an invalid lastRunAt date-time")
        })?;
    }
    Ok(())
}
fn protocol_error(message: &str) -> ClientError {
    ClientError::Protocol {
        diagnostic: None,
        message: message.into(),
        success_status_received: true,
    }
}
