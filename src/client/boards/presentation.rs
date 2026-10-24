use crate::client::{ClientError, WekanClient, transport::decode_success};
use reqwest::header::ACCEPT;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardViewVisibility {
    pub show_on_public: bool,
    pub show_on_private: bool,
}
/// Stored view overrides may contain either flag; GET's effective view has both.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct StoredBoardViewVisibility {
    pub show_on_public: Option<bool>,
    pub show_on_private: Option<bool>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardViewSettings {
    pub board_view_settings: BTreeMap<String, BoardViewVisibility>,
    pub default_public_board_view: String,
    pub default_private_board_view: String,
    pub board_view_order: Vec<String>,
    pub keys: Vec<String>,
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BoardViewVisibilityPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_on_public: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_on_private: Option<bool>,
}
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBoardViewSettingsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board_view_settings: Option<BTreeMap<String, BoardViewVisibilityPatch>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_public_board_view: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_private_board_view: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board_view_order: Option<Vec<String>>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all(deserialize = "camelCase", serialize = "snake_case")
)]
pub struct BoardCardFieldOrder {
    pub card_field_order: Vec<String>,
    pub keys: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CardFieldOrderRequest<'a> {
    card_field_order: &'a [String],
}

impl WekanClient {
    pub async fn board_view_settings(
        &self,
        board: &str,
        token: &SecretString,
    ) -> Result<BoardViewSettings, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.board_endpoint(board, Some("boardViewSettings"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "board view settings", true)
    }
    pub async fn update_board_view_settings(
        &self,
        board: &str,
        request: &UpdateBoardViewSettingsRequest,
        token: &SecretString,
    ) -> Result<BoardViewSettings, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.board_endpoint(board, Some("boardViewSettings"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(request),
            )
            .await?;
        decode_success(&body, "board view settings update", true)
    }
    pub async fn board_card_field_order(
        &self,
        board: &str,
        token: &SecretString,
    ) -> Result<BoardCardFieldOrder, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .get(self.board_endpoint(board, Some("cardFieldOrder"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret()),
            )
            .await?;
        decode_success(&body, "board card field order", true)
    }
    pub async fn update_board_card_field_order(
        &self,
        board: &str,
        fields: &[String],
        token: &SecretString,
    ) -> Result<BoardCardFieldOrder, ClientError> {
        let body = self
            .execute_success_request(
                self.http
                    .put(self.board_endpoint(board, Some("cardFieldOrder"))?)
                    .header(ACCEPT, "application/json")
                    .bearer_auth(token.expose_secret())
                    .json(&CardFieldOrderRequest {
                        card_field_order: fields,
                    }),
            )
            .await?;
        decode_success(&body, "board card field order update", true)
    }
}
