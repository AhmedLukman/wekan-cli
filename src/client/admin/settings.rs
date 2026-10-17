use super::present;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GlobalSettingsDocument {
    #[serde(default, deserialize_with = "present")]
    #[serde(rename = "_id")]
    pub settings_id: Option<String>,
    pub disable_registration: Option<bool>,
    pub disable_forgot_password: Option<bool>,
    pub product_name: Option<String>,
    pub display_authentication_method: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub default_authentication_method: Option<String>,
    pub spinner_name: Option<String>,
    pub hide_logo: Option<bool>,
    pub hide_card_counter_list: Option<bool>,
    pub hide_board_member_list: Option<bool>,
    pub custom_login_logo_image_url: Option<String>,
    pub custom_login_logo_link_url: Option<String>,
    pub custom_help_link_url: Option<String>,
    pub text_below_custom_login_logo: Option<String>,
    pub automatic_linked_url_schemes: Option<String>,
    pub custom_top_left_corner_logo_image_url: Option<String>,
    pub custom_top_left_corner_logo_link_url: Option<String>,
    pub custom_top_left_corner_logo_height: Option<String>,
    pub oidc_btn_text: Option<String>,
    pub mail_domain_name: Option<String>,
    pub legal_notice: Option<String>,
    pub custom_head_enabled: Option<bool>,
    pub custom_head_meta_tags: Option<String>,
    pub custom_head_link_tags: Option<String>,
    pub custom_manifest_enabled: Option<bool>,
    pub custom_manifest_content: Option<String>,
    pub custom_asset_links_enabled: Option<bool>,
    pub custom_asset_links_content: Option<String>,
    pub accessibility_page_enabled: Option<bool>,
    pub accessibility_title: Option<String>,
    pub accessibility_content: Option<String>,
    pub support_popup_text: Option<String>,
    pub support_page_enabled: Option<bool>,
    pub support_page_public: Option<bool>,
    pub support_title: Option<String>,
    pub support_page_text: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateGlobalSettings {
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub disable_registration: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub disable_forgot_password: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub product_name: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub display_authentication_method: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub default_authentication_method: Option<String>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub spinner_name: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub hide_logo: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub hide_card_counter_list: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub hide_board_member_list: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_login_logo_image_url: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_login_logo_link_url: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_help_link_url: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub text_below_custom_login_logo: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub automatic_linked_url_schemes: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_top_left_corner_logo_image_url: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_top_left_corner_logo_link_url: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_top_left_corner_logo_height: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub oidc_btn_text: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub mail_domain_name: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub legal_notice: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_head_enabled: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_head_meta_tags: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_head_link_tags: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_manifest_enabled: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_manifest_content: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_asset_links_enabled: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_asset_links_content: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub accessibility_page_enabled: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub accessibility_title: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub accessibility_content: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub support_popup_text: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub support_page_enabled: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub support_page_public: Option<Option<bool>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub support_title: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub support_page_text: Option<Option<String>>,
}

fn present_nullable<'de, D: serde::Deserializer<'de>, T: serde::Deserialize<'de>>(
    d: D,
) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(d).map(Some)
}
