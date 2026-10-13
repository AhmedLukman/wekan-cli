use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use super::{
    CardCustomField, CardDocument, protocol_error, require_non_empty, validate_card_document,
};
use crate::client::{
    ClientError, WekanClient,
    custom_fields::{custom_field_input_error, validate_custom_field_identifier},
    transport::decode_success,
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct SetCardCustomFieldRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SetCardCustomFieldResponse {
    #[serde(rename = "_id")]
    pub card_id: String,
    pub custom_fields: Vec<CardCustomField>,
}

pub(crate) fn validate_custom_field_query_value(value: &str) -> Result<(), &'static str> {
    match value {
        "" => Err("the custom-field lookup value must not be empty"),
        "." | ".." => Err(
            "dot-only custom-field lookup values cannot be sent by the HTTP client; no request was sent",
        ),
        _ => Ok(()),
    }
}

impl WekanClient {
    pub async fn set_card_custom_field(
        &self,
        board: &str,
        list: &str,
        card: &str,
        field: &str,
        request: &SetCardCustomFieldRequest,
        token: &SecretString,
    ) -> Result<SetCardCustomFieldResponse, ClientError> {
        for id in [board, list, card, field] {
            validate_custom_field_identifier(id).map_err(custom_field_input_error)?;
        }
        let mut endpoint = self.card_endpoint(board, list, Some(card))?;
        endpoint
            .path_segments_mut()
            .map_err(|()| protocol_error("could not add the custom-field selector".to_owned()))?
            .extend(["customFields", field]);
        let body = self
            .execute_success_request(
                self.http
                    .post(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        let response: SetCardCustomFieldResponse =
            decode_success(&body, "card custom-field assignment", true)?;
        if response.card_id != card {
            return Err(protocol_error(
                "Wekan acknowledged a different card than requested".to_owned(),
            ));
        }
        require_non_empty(&response.card_id, "card custom-field assignment", "card id")?;
        Ok(response)
    }

    pub async fn cards_by_custom_field(
        &self,
        board: &str,
        field: &str,
        value: &str,
        token: &SecretString,
    ) -> Result<Vec<CardDocument>, ClientError> {
        for id in [board, field] {
            validate_custom_field_identifier(id).map_err(custom_field_input_error)?;
        }
        validate_custom_field_query_value(value).map_err(|message| ClientError::Protocol {
            diagnostic: None,
            message: message.to_owned(),
            success_status_received: false,
        })?;
        let mut endpoint = self.server().join("api/boards").map_err(|_| {
            protocol_error("could not build the custom-field card query".to_owned())
        })?;
        endpoint
            .path_segments_mut()
            .map_err(|()| protocol_error("could not add custom-field query selectors".to_owned()))?
            .extend([board, "cardsByCustomField", field, value]);
        let body = self
            .execute_success_request(
                self.http
                    .get(endpoint)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let cards: Vec<CardDocument> = decode_success(&body, "custom-field card collection", true)?;
        cards.into_iter().map(validate_card_document).collect()
    }
}
