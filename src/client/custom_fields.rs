use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use super::{
    ClientError, WekanClient, datetime::validate_response_date, transport::decode_success,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CustomFieldType {
    Text,
    Number,
    Date,
    Dropdown,
    #[serde(rename = "dropdownMultiSelect")]
    DropdownMultiSelect,
    Checkbox,
    Currency,
    Stringtemplate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomFieldSummary {
    #[serde(rename = "_id")]
    pub custom_field_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: CustomFieldType,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomFieldDropdownItem {
    #[serde(rename = "_id")]
    pub item_id: String,
    pub name: String,
}

/// Optional properties retain omission separately from an explicit JSON null.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CustomFieldSettings {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable_setting"
    )]
    pub jira_time_field: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable_setting"
    )]
    pub jira_estimate_field_id: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable_setting"
    )]
    pub jira_estimate_unit: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable_setting"
    )]
    pub currency_code: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable_items"
    )]
    pub dropdown_items: Option<Option<Vec<CustomFieldDropdownItem>>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable_setting"
    )]
    pub stringtemplate_format: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "nullable_setting"
    )]
    pub stringtemplate_separator: Option<Option<String>>,
}

impl CustomFieldSettings {
    pub(crate) fn validate_dropdown_item_ids(&self) -> Result<(), &'static str> {
        if let Some(Some(items)) = &self.dropdown_items {
            for item in items {
                // Wekan trims these IDs before storing them. Validate the cleaned
                // value without changing the request or restricting read DTOs.
                let id = super::strings::trim_javascript_whitespace(&item.item_id);
                validate_custom_field_identifier(id).map_err(|_| {
                    "dropdown item IDs must not be dot-only after Wekan trims whitespace; no request was sent"
                })?;
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_custom_field_identifier(value: &str) -> Result<(), &'static str> {
    match value {
        "." | ".." => {
            Err("dot-only identifiers cannot be sent by the HTTP client; no request was sent")
        }
        _ => Ok(()),
    }
}

pub(crate) fn custom_field_input_error(message: &str) -> ClientError {
    ClientError::Protocol {
        diagnostic: None,
        message: message.to_owned(),
        success_status_received: false,
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CustomFieldDocument {
    #[serde(rename = "_id")]
    pub custom_field_id: String,
    pub board_ids: Vec<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: CustomFieldType,
    #[serde(deserialize_with = "settings_object")]
    pub settings: CustomFieldSettings,
    pub show_on_card: bool,
    pub automatically_on_card: bool,
    pub always_on_card: bool,
    pub show_label_on_mini_card: bool,
    pub show_sum_at_top_of_list: bool,
    pub created_at: Option<String>,
    pub modified_at: String,
    pub card_kinds: Option<Vec<String>>,
    pub read_only: Option<bool>,
    pub admin_only: Option<bool>,
    pub sort: Option<serde_json::Number>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCustomFieldRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: CustomFieldType,
    pub settings: CustomFieldSettings,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_on_card: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatically_on_card: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_label_on_mini_card: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_sum_at_top_of_list: Option<bool>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCustomFieldRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub field_type: Option<CustomFieldType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<CustomFieldSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_on_card: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatically_on_card: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub always_on_card: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_label_on_mini_card: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_sum_at_top_of_list: Option<bool>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct CreateCustomFieldItemsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct UpdateCustomFieldItemRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomFieldIdResponse {
    #[serde(rename = "_id")]
    pub id: String,
}

impl WekanClient {
    pub async fn board_custom_fields(
        &self,
        board: &str,
        token: &SecretString,
    ) -> Result<Vec<CustomFieldSummary>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.custom_field_endpoint(board, None, false, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        let fields: Vec<CustomFieldSummary> =
            decode_success(&body, "custom-field collection", true)?;
        for field in &fields {
            require_id(&field.custom_field_id)?;
        }
        Ok(fields)
    }

    pub async fn custom_field(
        &self,
        board: &str,
        field: &str,
        token: &SecretString,
    ) -> Result<Option<CustomFieldDocument>, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.custom_field_endpoint(board, Some(field), false, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        if body.is_empty() {
            // The pinned route sends HTTP 200 with no data when its scoped lookup misses.
            return Ok(None);
        }
        let document: CustomFieldDocument = decode_success(&body, "custom field", true)?;
        require_id(&document.custom_field_id)?;
        if document.custom_field_id != field || !document.board_ids.iter().any(|id| id == board) {
            return Err(protocol_error(
                "the custom-field response identifiers did not match the requested scope",
            ));
        }
        for id in &document.board_ids {
            require_id(id)?;
        }
        if let Some(Some(items)) = &document.settings.dropdown_items {
            for item in items {
                require_id(&item.item_id)?;
            }
        }
        for date in std::iter::once(&document.modified_at).chain(document.created_at.iter()) {
            validate_response_date(date).map_err(|()| {
                protocol_error("the custom-field response contained an invalid date-time")
            })?;
        }
        Ok(Some(document))
    }

    pub async fn create_custom_field(
        &self,
        board: &str,
        request: &CreateCustomFieldRequest,
        token: &SecretString,
    ) -> Result<CustomFieldIdResponse, ClientError> {
        request
            .settings
            .validate_dropdown_item_ids()
            .map_err(custom_field_input_error)?;
        let body = self
            .execute_success_request(
                self.http
                    .post(self.custom_field_endpoint(board, None, false, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        decode_id(&body, None)
    }

    pub async fn update_custom_field(
        &self,
        board: &str,
        field: &str,
        request: &UpdateCustomFieldRequest,
        token: &SecretString,
    ) -> Result<CustomFieldIdResponse, ClientError> {
        if let Some(settings) = &request.settings {
            settings
                .validate_dropdown_item_ids()
                .map_err(custom_field_input_error)?;
        }
        let body = self
            .execute_success_request(
                self.http
                    .put(self.custom_field_endpoint(board, Some(field), false, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        decode_id(&body, Some(field))
    }

    pub async fn delete_custom_field(
        &self,
        board: &str,
        field: &str,
        token: &SecretString,
    ) -> Result<CustomFieldIdResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .delete(self.custom_field_endpoint(board, Some(field), false, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_id(&body, Some(field))
    }

    pub async fn create_custom_field_items(
        &self,
        board: &str,
        field: &str,
        request: &CreateCustomFieldItemsRequest,
        token: &SecretString,
    ) -> Result<CustomFieldIdResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .post(self.custom_field_endpoint(board, Some(field), true, None)?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        decode_id(&body, Some(field))
    }

    pub async fn update_custom_field_item(
        &self,
        board: &str,
        field: &str,
        item: &str,
        request: &UpdateCustomFieldItemRequest,
        token: &SecretString,
    ) -> Result<CustomFieldIdResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.custom_field_endpoint(board, Some(field), true, Some(item))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        decode_id(&body, Some(item))
    }

    pub async fn delete_custom_field_item(
        &self,
        board: &str,
        field: &str,
        item: &str,
        token: &SecretString,
    ) -> Result<CustomFieldIdResponse, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .delete(self.custom_field_endpoint(board, Some(field), true, Some(item))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        // The pinned route acknowledges the parent field, not the dropdown item.
        decode_id(&body, Some(field))
    }

    fn custom_field_endpoint(
        &self,
        board: &str,
        field: Option<&str>,
        dropdown: bool,
        item: Option<&str>,
    ) -> Result<reqwest::Url, ClientError> {
        for id in [Some(board), field, item].into_iter().flatten() {
            validate_custom_field_identifier(id).map_err(custom_field_input_error)?;
        }
        let mut endpoint = self
            .server()
            .join("api/boards")
            .map_err(|_| ClientError::Protocol {
                diagnostic: None,
                message: "could not build the custom-field endpoint".to_owned(),
                success_status_received: false,
            })?;
        let mut segments = endpoint
            .path_segments_mut()
            .map_err(|()| ClientError::Protocol {
                diagnostic: None,
                message: "could not add custom-field identifiers".to_owned(),
                success_status_received: false,
            })?;
        segments.extend([board, "custom-fields"]);
        if let Some(field) = field {
            segments.push(field);
        }
        if dropdown {
            segments.push("dropdown-items");
        }
        if let Some(item) = item {
            segments.push(item);
        }
        drop(segments);
        Ok(endpoint)
    }
}

fn decode_id(body: &[u8], expected: Option<&str>) -> Result<CustomFieldIdResponse, ClientError> {
    let response: CustomFieldIdResponse =
        decode_success(body, "custom-field acknowledgement", true)?;
    require_id(&response.id)?;
    if expected.is_some_and(|id| id != response.id) {
        return Err(protocol_error(
            "Wekan acknowledged a different custom-field identifier than requested",
        ));
    }
    Ok(response)
}

fn require_id(id: &str) -> Result<(), ClientError> {
    if super::strings::trim_javascript_whitespace(id).is_empty() {
        Err(protocol_error(
            "the custom-field response contained an empty identifier",
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

fn nullable_setting<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(deserializer).map(Some)
}

struct Object<T>(T);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
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
        deserializer.deserialize_map(Visitor(std::marker::PhantomData))
    }
}
fn settings_object<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<CustomFieldSettings, D::Error> {
    Object::<CustomFieldSettings>::deserialize(deserializer).map(|value| value.0)
}
fn nullable_items<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<Vec<CustomFieldDropdownItem>>>, D::Error> {
    Option::<Vec<Object<CustomFieldDropdownItem>>>::deserialize(deserializer)
        .map(|items| Some(items.map(|items| items.into_iter().map(|item| item.0).collect())))
}
