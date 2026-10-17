use reqwest::{Method, header::ACCEPT};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize, Serializer};

use super::{
    ClientError, WekanClient, datetime::validate_response_date, transport::decode_success,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct IntegrationDocument {
    #[serde(rename = "_id")]
    pub integration_id: String,
    pub enabled: bool,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub integration_type: String,
    pub activities: Vec<String>,
    pub url: String,
    pub board_id: String,
    pub created_at: String,
    pub modified_at: String,
    pub user_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IntegrationIdResponse {
    #[serde(rename = "_id")]
    pub integration_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IntegrationActivitiesResponse {
    #[serde(rename = "_id")]
    pub integration_id: String,
    pub activities: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateIntegrationRequest {
    pub url: String,
}

#[derive(Debug, Default, Serialize)]
pub struct UpdateIntegrationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_token"
    )]
    pub token: Option<SecretString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activities: Option<Vec<String>>,
}

fn serialize_token<S: Serializer>(token: &Option<SecretString>, s: S) -> Result<S::Ok, S::Error> {
    token.as_ref().map(ExposeSecret::expose_secret).serialize(s)
}

#[derive(Clone, Debug, Serialize)]
pub struct IntegrationActivitiesRequest {
    pub activities: Vec<String>,
}

impl WekanClient {
    pub async fn board_integrations(
        &self,
        board: &str,
        token: &SecretString,
    ) -> Result<Vec<IntegrationDocument>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.integration_endpoint(board, None, false)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let documents: Vec<IntegrationDocument> =
            decode_success(&body, "integration collection", true)?;
        for document in &documents {
            validate_document(document, board, None)?;
        }
        Ok(documents)
    }

    pub async fn integration(
        &self,
        board: &str,
        integration: &str,
        token: &SecretString,
    ) -> Result<Option<IntegrationDocument>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.integration_endpoint(board, Some(integration), false)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        if body.is_empty() {
            return Ok(None);
        }
        let document: IntegrationDocument = decode_success(&body, "integration lookup", true)?;
        validate_document(&document, board, Some(integration))?;
        Ok(Some(document))
    }

    pub async fn create_integration(
        &self,
        board: &str,
        request: &CreateIntegrationRequest,
        token: &SecretString,
    ) -> Result<IntegrationIdResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.integration_endpoint(board, None, false)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        decode_id(&body, None)
    }

    pub async fn update_integration(
        &self,
        board: &str,
        integration: &str,
        request: &UpdateIntegrationRequest,
        token: &SecretString,
    ) -> Result<IntegrationIdResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.integration_endpoint(board, Some(integration), false)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        decode_id(&body, Some(integration))
    }

    pub async fn delete_integration(
        &self,
        board: &str,
        integration: &str,
        token: &SecretString,
    ) -> Result<IntegrationIdResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .delete(self.integration_endpoint(board, Some(integration), false)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_id(&body, Some(integration))
    }

    pub async fn add_integration_activities(
        &self,
        board: &str,
        integration: &str,
        request: &IntegrationActivitiesRequest,
        token: &SecretString,
    ) -> Result<Option<IntegrationActivitiesResponse>, ClientError> {
        self.mutate_integration_activities(Method::POST, board, integration, request, token)
            .await
    }

    pub async fn remove_integration_activities(
        &self,
        board: &str,
        integration: &str,
        request: &IntegrationActivitiesRequest,
        token: &SecretString,
    ) -> Result<Option<IntegrationActivitiesResponse>, ClientError> {
        self.mutate_integration_activities(Method::DELETE, board, integration, request, token)
            .await
    }

    async fn mutate_integration_activities(
        &self,
        method: Method,
        board: &str,
        integration: &str,
        request: &IntegrationActivitiesRequest,
        token: &SecretString,
    ) -> Result<Option<IntegrationActivitiesResponse>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .request(
                        method,
                        self.integration_endpoint(board, Some(integration), true)?,
                    )
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        if body.is_empty() {
            return Ok(None);
        }
        let response: IntegrationActivitiesResponse =
            decode_success(&body, "integration activities", true)?;
        validate_response_id(&response.integration_id, Some(integration))?;
        Ok(Some(response))
    }

    fn integration_endpoint(
        &self,
        board: &str,
        integration: Option<&str>,
        activities: bool,
    ) -> Result<reqwest::Url, ClientError> {
        for id in [Some(board), integration].into_iter().flatten() {
            if id.trim().is_empty() || matches!(id, "." | "..") {
                return Err(protocol_error(
                    "integration endpoint IDs must be nonempty and not dot-only; no request was sent",
                    false,
                ));
            }
        }
        let mut endpoint = self
            .server()
            .join("api/boards")
            .map_err(|_| protocol_error("could not build the integration endpoint", false))?;
        let mut segments = endpoint.path_segments_mut().map_err(|()| {
            protocol_error("could not add integration endpoint identifiers", false)
        })?;
        segments.extend([board, "integrations"]);
        if let Some(integration) = integration {
            segments.push(integration);
        }
        if activities {
            segments.push("activities");
        }
        drop(segments);
        Ok(endpoint)
    }
}

fn validate_document(
    document: &IntegrationDocument,
    board: &str,
    integration: Option<&str>,
) -> Result<(), ClientError> {
    validate_response_id(&document.integration_id, integration)?;
    for id in [&document.board_id, &document.user_id] {
        validate_response_id(id, None)?;
    }
    if document.board_id != board {
        return Err(protocol_error(
            "the integration response board ID did not match the requested scope",
            true,
        ));
    }
    for date in [&document.created_at, &document.modified_at] {
        validate_response_date(date).map_err(|()| {
            protocol_error(
                "the integration response contained an invalid date-time",
                true,
            )
        })?;
    }
    Ok(())
}

fn decode_id(body: &[u8], expected: Option<&str>) -> Result<IntegrationIdResponse, ClientError> {
    let response: IntegrationIdResponse =
        decode_success(body, "integration acknowledgement", true)?;
    validate_response_id(&response.integration_id, expected)?;
    Ok(response)
}

fn validate_response_id(id: &str, expected: Option<&str>) -> Result<(), ClientError> {
    if id.trim().is_empty() {
        return Err(protocol_error(
            "the integration response contained an empty identifier",
            true,
        ));
    }
    if expected.is_some_and(|expected| expected != id) {
        return Err(protocol_error(
            "Wekan returned a different integration ID than requested",
            true,
        ));
    }
    Ok(())
}

fn protocol_error(message: &str, success_status_received: bool) -> ClientError {
    ClientError::Protocol {
        diagnostic: None,
        message: message.to_owned(),
        success_status_received,
    }
}
