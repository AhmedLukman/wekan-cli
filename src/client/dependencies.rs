use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Number;

use super::{ClientError, WekanClient, transport::decode_success};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyType {
    RelatedTo,
    Blocks,
    IsBlockedBy,
    Fixes,
    IsFixedBy,
}

impl DependencyType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RelatedTo => "related-to",
            Self::Blocks => "blocks",
            Self::IsBlockedBy => "is-blocked-by",
            Self::Fixes => "fixes",
            Self::IsFixedBy => "is-fixed-by",
        }
    }
}

/// Normalized endpoint response, distinct from raw cardDependencies entries.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CardDependencySummary {
    #[serde(rename = "cardId")]
    pub target_card_id: String,
    #[serde(rename = "type")]
    pub dependency_type: DependencyType,
    pub color: String,
    pub icon: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BoardDependencyLine {
    #[serde(rename = "from")]
    pub source_card_id: String,
    #[serde(rename = "fromTitle")]
    pub source_title: Option<String>,
    #[serde(rename = "fromCardNumber")]
    pub source_card_number: Option<Number>,
    #[serde(rename = "to")]
    pub target_card_id: String,
    #[serde(rename = "toTitle")]
    pub target_title: Option<String>,
    #[serde(rename = "toCardNumber")]
    pub target_card_number: Option<Number>,
    #[serde(rename = "type")]
    pub dependency_type: DependencyType,
    pub color: String,
    pub icon: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateDependencyResponse {
    #[serde(rename = "_id")]
    pub card_id: String,
    #[serde(rename = "cardId")]
    pub target_card_id: String,
    #[serde(rename = "type")]
    pub dependency_type: DependencyType,
    pub color: String,
    pub icon: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DependencyMutationResponse {
    #[serde(rename = "_id")]
    pub card_id: String,
}

/// None omits a property, Some(None) sends null, Some(Some(value)) sends the string.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct DependencyMetadata {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub dependency_type: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Option<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CreateDependencyRequest {
    #[serde(rename = "cardId")]
    pub target_card_id: String,
    #[serde(flatten)]
    pub metadata: DependencyMetadata,
}

impl WekanClient {
    pub async fn card_dependencies(
        &self,
        board: &str,
        card: &str,
        token: &SecretString,
    ) -> Result<Vec<CardDependencySummary>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.dependency_endpoint(board, Some(card), None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let dependencies: Vec<CardDependencySummary> =
            decode_success(&body, "card dependencies", true)?;
        for dependency in &dependencies {
            require_id(&dependency.target_card_id)?;
        }
        Ok(dependencies)
    }

    pub async fn board_dependencies(
        &self,
        board: &str,
        token: &SecretString,
    ) -> Result<Vec<BoardDependencyLine>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.dependency_endpoint(board, None, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let dependencies: Vec<BoardDependencyLine> =
            decode_success(&body, "board dependencies", true)?;
        for dependency in &dependencies {
            require_id(&dependency.source_card_id)?;
            require_id(&dependency.target_card_id)?;
        }
        Ok(dependencies)
    }

    pub async fn create_dependency(
        &self,
        board: &str,
        card: &str,
        request: &CreateDependencyRequest,
        token: &SecretString,
    ) -> Result<CreateDependencyResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.dependency_endpoint(board, Some(card), None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: CreateDependencyResponse =
            decode_success(&body, "dependency creation", true)?;
        require_id(&response.card_id)?;
        require_id(&response.target_card_id)?;
        if response.card_id != card || response.target_card_id != request.target_card_id {
            return Err(protocol_error(
                "Wekan acknowledged a dependency with different source or target IDs",
            ));
        }
        Ok(response)
    }

    pub async fn update_dependency(
        &self,
        board: &str,
        card: &str,
        target: &str,
        request: &DependencyMetadata,
        token: &SecretString,
    ) -> Result<DependencyMutationResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.dependency_endpoint(board, Some(card), Some(target))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        decode_acknowledgment(&body, card)
    }

    pub async fn delete_dependency(
        &self,
        board: &str,
        card: &str,
        target: &str,
        token: &SecretString,
    ) -> Result<DependencyMutationResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .delete(self.dependency_endpoint(board, Some(card), Some(target))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_acknowledgment(&body, card)
    }

    fn dependency_endpoint(
        &self,
        board: &str,
        card: Option<&str>,
        target: Option<&str>,
    ) -> Result<reqwest::Url, ClientError> {
        // URL path segments cannot represent dot-only IDs without normalization.
        for id in [Some(board), card, target].into_iter().flatten() {
            if id.trim().is_empty() || matches!(id, "." | "..") {
                return Err(ClientError::Protocol { diagnostic: None,
                    message: "dependency endpoint IDs must be nonempty and not dot-only; no request was sent".to_owned(),
                    success_status_received: false });
            }
        }
        let mut endpoint = self
            .server()
            .join("api/boards")
            .map_err(|_| ClientError::Protocol {
                diagnostic: None,
                message: "could not build the dependency endpoint".to_owned(),
                success_status_received: false,
            })?;
        let mut segments = endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                diagnostic: None,
                message: "could not add dependency endpoint identifiers".to_owned(),
                success_status_received: false,
            })?;
        segments.push(board);
        if let Some(card) = card {
            segments.extend(["cards", card]);
        }
        segments.push("dependencies");
        if let Some(target) = target {
            segments.push(target);
        }
        drop(segments);
        Ok(endpoint)
    }
}

fn decode_acknowledgment(
    body: &[u8],
    card: &str,
) -> Result<DependencyMutationResponse, ClientError> {
    let response: DependencyMutationResponse = decode_success(body, "dependency mutation", true)?;
    require_id(&response.card_id)?;
    if response.card_id != card {
        return Err(protocol_error(
            "Wekan acknowledged a different source card ID than requested",
        ));
    }
    Ok(response)
}

fn require_id(id: &str) -> Result<(), ClientError> {
    if id.trim().is_empty() {
        Err(protocol_error(
            "the dependency response contained an empty card ID",
        ))
    } else {
        Ok(())
    }
}

fn protocol_error(message: &str) -> ClientError {
    ClientError::Protocol {
        diagnostic: None,
        message: message.to_owned(),
        success_status_received: true,
    }
}
