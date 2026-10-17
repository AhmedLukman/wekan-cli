use reqwest::{StatusCode, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize, Serializer};

use crate::client::{
    ClientError, WekanClient,
    decoding::{decode_value, parse_json},
    transport::{decode_success, embedded_error},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatedUserToken {
    #[serde(rename = "_id")]
    pub user_id: String,
    #[serde(rename = "authToken", deserialize_with = "deserialize_secret")]
    pub auth_token: SecretString,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteUserTokenRequest {
    pub user_id: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_token"
    )]
    pub token: Option<SecretString>,
}

fn serialize_token<S: Serializer>(value: &Option<SecretString>, s: S) -> Result<S::Ok, S::Error> {
    value.as_ref().map(ExposeSecret::expose_secret).serialize(s)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserTokenDeletion {
    // Wekan includes the supplied token in this message. Keep Debug secret-safe.
    #[serde(deserialize_with = "deserialize_secret")]
    pub message: SecretString,
}

fn deserialize_secret<'de, D: serde::Deserializer<'de>>(d: D) -> Result<SecretString, D::Error> {
    String::deserialize(d).map(SecretString::from)
}

impl WekanClient {
    pub async fn create_user_token(
        &self,
        user_id: &str,
        credential: &SecretString,
    ) -> Result<CreatedUserToken, ClientError> {
        let mut endpoint = self.endpoint("api/createtoken", "token creation")?;
        endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                diagnostic: None,
                message: "could not add the token user ID to the endpoint".to_owned(),
                success_status_received: false,
            })?
            .push(user_id);
        let body = self
            .execute_success_request(
                self.http
                    .post(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(credential.expose_secret()),
            )
            .await?;
        let response: CreatedUserToken = decode_success(&body, "token creation", true)?;
        if response.user_id.is_empty()
            || response.user_id != user_id
            || response.auth_token.expose_secret().is_empty()
        {
            return Err(ClientError::Protocol {
                diagnostic: None,
                message:
                    "the token creation response contained an empty token or an unexpected user ID"
                        .to_owned(),
                success_status_received: true,
            });
        }
        Ok(response)
    }

    pub async fn delete_user_token(
        &self,
        request: &DeleteUserTokenRequest,
        credential: &SecretString,
    ) -> Result<UserTokenDeletion, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.endpoint("api/deletetoken", "token deletion")?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(credential.expose_secret())
                    .json(request),
            )
            .await?;
        let value = parse_json(&body, "token deletion")?;
        // A message-only object is this route's success shape. The generic
        // decoder treats message as an embedded-error marker on other routes.
        let success = decode_value::<UserTokenDeletion>(value.clone(), "token deletion", true);
        let response = match success {
            Ok(response) => response,
            Err(error) => {
                if value.as_object().is_some_and(|o| {
                    ["error", "reason", "statusCode", "errorType"]
                        .iter()
                        .any(|key| o.contains_key(*key))
                }) {
                    if let Some(embedded) = embedded_error(&value, StatusCode::OK)? {
                        return Err(embedded);
                    }
                }
                return Err(error);
            }
        };
        Ok(response)
    }
}
