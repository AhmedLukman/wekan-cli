use super::present;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GlobalSettingsDocument {
    #[serde(default, deserialize_with = "present")]
    #[serde(rename = "_id")]
    pub settings_id: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub disable_registration: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub disable_forgot_password: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub product_name: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub display_authentication_method: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub default_authentication_method: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub spinner_name: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub hide_logo: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub hide_card_counter_list: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub hide_board_member_list: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub custom_login_logo_image_url: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub custom_login_logo_link_url: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub custom_help_link_url: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub text_below_custom_login_logo: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub automatic_linked_url_schemes: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub custom_top_left_corner_logo_image_url: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub custom_top_left_corner_logo_link_url: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub custom_top_left_corner_logo_height: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub oidc_btn_text: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub mail_domain_name: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub legal_notice: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub custom_head_enabled: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub custom_head_meta_tags: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub custom_head_link_tags: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub custom_manifest_enabled: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub custom_manifest_content: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub custom_asset_links_enabled: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub custom_asset_links_content: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub accessibility_page_enabled: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub accessibility_title: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub accessibility_content: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub support_popup_text: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub support_page_enabled: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub support_page_public: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub support_title: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub support_page_text: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateGlobalSettings {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub disable_registration: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub disable_forgot_password: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub product_name: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub display_authentication_method: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub default_authentication_method: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub spinner_name: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub hide_logo: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub hide_card_counter_list: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub hide_board_member_list: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_login_logo_image_url: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_login_logo_link_url: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_help_link_url: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub text_below_custom_login_logo: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub automatic_linked_url_schemes: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_top_left_corner_logo_image_url: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_top_left_corner_logo_link_url: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_top_left_corner_logo_height: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub oidc_btn_text: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub mail_domain_name: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub legal_notice: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_head_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_head_meta_tags: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_head_link_tags: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_manifest_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_manifest_content: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_asset_links_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_asset_links_content: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub accessibility_page_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub accessibility_title: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub accessibility_content: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub support_popup_text: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub support_page_enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub support_page_public: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub support_title: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub support_page_text: Option<String>,
}
