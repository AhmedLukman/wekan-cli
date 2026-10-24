//! Native incoming-email hooks use provider signatures rather than a Wekan session.
use crate::client::{ClientError, WekanClient, transport::decode_success};
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InboundEmailReceipt {
    #[serde(
        rename(deserialize = "_id", serialize = "resource_id"),
        deserialize_with = "nullable_id"
    )]
    pub resource_id: Option<String>,
}
fn nullable_id<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::deserialize(d)
}
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BounceReceipt {
    pub ok: bool,
    pub recorded: u64,
}
impl WekanClient {
    async fn email_hook(
        &self,
        provider: Option<&str>,
        body: Vec<u8>,
        content_type: &str,
        headers: HeaderMap,
    ) -> Result<Vec<u8>, ClientError> {
        let mut url = self
            .server()
            .join("api/inbound-email")
            .map_err(|_| super::protocol_error("could not build inbound email endpoint", true))?;
        if let Some(provider) = provider {
            url.path_segments_mut()
                .map_err(|_| super::protocol_error("could not build bounce endpoint", true))?
                .extend(["bounces", provider]);
        }
        self.execute_success_request(
            self.http
                .post(url)
                .headers(headers)
                .header(ACCEPT, "application/json")
                .header(CONTENT_TYPE, content_type)
                .body(body),
        )
        .await
    }
    pub async fn submit_inbound_email(
        &self,
        body: Vec<u8>,
        content_type: &str,
        headers: HeaderMap,
    ) -> Result<InboundEmailReceipt, ClientError> {
        decode_success(
            &self.email_hook(None, body, content_type, headers).await?,
            "inbound email",
            true,
        )
    }
    pub async fn submit_email_bounce(
        &self,
        provider: &str,
        body: Vec<u8>,
        content_type: &str,
        headers: HeaderMap,
    ) -> Result<BounceReceipt, ClientError> {
        let response: BounceReceipt = decode_success(
            &self
                .email_hook(Some(provider), body, content_type, headers)
                .await?,
            "email bounce",
            true,
        )?;
        if !response.ok {
            return Err(super::protocol_error(
                "email bounce did not return ok: true",
                true,
            ));
        }
        Ok(response)
    }
}
