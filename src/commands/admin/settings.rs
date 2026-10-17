use super::input;
use crate::client::admin::{PreparedSettingsUpdate, settings::UpdateGlobalSettings};
use crate::{
    client::WekanClientFactory,
    command_result::*,
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_client_error, map_partial_mutation_client_error_with_not_found},
    },
    credentials::CredentialStore,
    error::AppError,
    redaction::Redactor,
};
use clap::{ArgGroup, Args, Subcommand};
#[derive(Debug, Args)]
pub struct SettingsArgs {
    #[command(subcommand)]
    pub command: SettingsCommand,
}
#[derive(Debug, Subcommand)]
pub enum SettingsCommand {
    /// Return the complete mapped settings document.
    Get,
    /// Apply native settings updates, with no read-before-write merging.
    Update(Box<UpdateArgs>),
}
#[derive(Debug, Args)]
#[command(group(ArgGroup::new("setting_flags").args([
    "disable_registration",
    "disable_forgot_password",
    "product_name",
    "display_authentication_method",
    "default_authentication_method",
    "spinner_name",
    "hide_logo",
    "hide_card_counter_list",
    "hide_board_member_list",
    "custom_login_logo_image_url",
    "custom_login_logo_link_url",
    "custom_help_link_url",
    "text_below_custom_login_logo",
    "automatic_linked_url_schemes",
    "custom_top_left_corner_logo_image_url",
    "custom_top_left_corner_logo_link_url",
    "custom_top_left_corner_logo_height",
    "oidc_btn_text",
    "mail_domain_name",
    "legal_notice",
    "custom_head_enabled",
    "custom_head_meta_tags",
    "custom_head_link_tags",
    "custom_manifest_enabled",
    "custom_manifest_content",
    "custom_asset_links_enabled",
    "custom_asset_links_content",
    "accessibility_page_enabled",
    "accessibility_title",
    "accessibility_content",
    "support_popup_text",
    "support_page_enabled",
    "support_page_public",
    "support_title",
    "support_page_text",
]).multiple(true).conflicts_with("settings_source")), group(ArgGroup::new("update_input").args([
    "settings_file",
    "settings_stdin",
    "disable_registration",
    "disable_forgot_password",
    "product_name",
    "display_authentication_method",
    "default_authentication_method",
    "spinner_name",
    "hide_logo",
    "hide_card_counter_list",
    "hide_board_member_list",
    "custom_login_logo_image_url",
    "custom_login_logo_link_url",
    "custom_help_link_url",
    "text_below_custom_login_logo",
    "automatic_linked_url_schemes",
    "custom_top_left_corner_logo_image_url",
    "custom_top_left_corner_logo_link_url",
    "custom_top_left_corner_logo_height",
    "oidc_btn_text",
    "mail_domain_name",
    "legal_notice",
    "custom_head_enabled",
    "custom_head_meta_tags",
    "custom_head_link_tags",
    "custom_manifest_enabled",
    "custom_manifest_content",
    "custom_asset_links_enabled",
    "custom_asset_links_content",
    "accessibility_page_enabled",
    "accessibility_title",
    "accessibility_content",
    "support_popup_text",
    "support_page_enabled",
    "support_page_public",
    "support_title",
    "support_page_text",
]).required(true).multiple(true)), after_help = "Only the v11.06 REST allowlist is supported; SMTP/mailServer is excluded. Omitted fields remain unchanged. JSON source options conflict with all setting flags.")]
pub struct UpdateArgs {
    #[command(flatten)]
    source: input::SourceArgs,
    /// Set disable registration. Explicit true or false.
    #[arg(long = "disable-registration")]
    disable_registration: Option<bool>,
    /// Set disable forgot password. Explicit true or false.
    #[arg(long = "disable-forgot-password")]
    disable_forgot_password: Option<bool>,
    /// Set product name.
    #[arg(long = "product-name")]
    product_name: Option<String>,
    /// Set display authentication method. Explicit true or false.
    #[arg(long = "display-authentication-method")]
    display_authentication_method: Option<bool>,
    /// Set default authentication method.
    #[arg(long = "default-authentication-method")]
    default_authentication_method: Option<String>,
    /// Set spinner name.
    #[arg(long = "spinner-name")]
    spinner_name: Option<String>,
    /// Set hide logo. Explicit true or false.
    #[arg(long = "hide-logo")]
    hide_logo: Option<bool>,
    /// Set hide card counter list. Explicit true or false.
    #[arg(long = "hide-card-counter-list")]
    hide_card_counter_list: Option<bool>,
    /// Set hide board member list. Explicit true or false.
    #[arg(long = "hide-board-member-list")]
    hide_board_member_list: Option<bool>,
    /// Set custom login logo image url.
    #[arg(long = "custom-login-logo-image-url")]
    custom_login_logo_image_url: Option<String>,
    /// Set custom login logo link url.
    #[arg(long = "custom-login-logo-link-url")]
    custom_login_logo_link_url: Option<String>,
    /// Set custom help link url.
    #[arg(long = "custom-help-link-url")]
    custom_help_link_url: Option<String>,
    /// Set text below custom login logo.
    #[arg(long = "text-below-custom-login-logo")]
    text_below_custom_login_logo: Option<String>,
    /// Set automatic linked url schemes.
    #[arg(long = "automatic-linked-url-schemes")]
    automatic_linked_url_schemes: Option<String>,
    /// Set custom top left corner logo image url.
    #[arg(long = "custom-top-left-corner-logo-image-url")]
    custom_top_left_corner_logo_image_url: Option<String>,
    /// Set custom top left corner logo link url.
    #[arg(long = "custom-top-left-corner-logo-link-url")]
    custom_top_left_corner_logo_link_url: Option<String>,
    /// Set custom top left corner logo height.
    #[arg(long = "custom-top-left-corner-logo-height")]
    custom_top_left_corner_logo_height: Option<String>,
    /// Set oidc btn text.
    #[arg(long = "oidc-btn-text")]
    oidc_btn_text: Option<String>,
    /// Set mail domain name.
    #[arg(long = "mail-domain-name")]
    mail_domain_name: Option<String>,
    /// Set legal notice.
    #[arg(long = "legal-notice")]
    legal_notice: Option<String>,
    /// Set custom head enabled. Explicit true or false.
    #[arg(long = "custom-head-enabled")]
    custom_head_enabled: Option<bool>,
    /// Set custom head meta tags.
    #[arg(long = "custom-head-meta-tags")]
    custom_head_meta_tags: Option<String>,
    /// Set custom head link tags.
    #[arg(long = "custom-head-link-tags")]
    custom_head_link_tags: Option<String>,
    /// Set custom manifest enabled. Explicit true or false.
    #[arg(long = "custom-manifest-enabled")]
    custom_manifest_enabled: Option<bool>,
    /// Set custom manifest content.
    #[arg(long = "custom-manifest-content")]
    custom_manifest_content: Option<String>,
    /// Set custom asset links enabled. Explicit true or false.
    #[arg(long = "custom-asset-links-enabled")]
    custom_asset_links_enabled: Option<bool>,
    /// Set custom asset links content.
    #[arg(long = "custom-asset-links-content")]
    custom_asset_links_content: Option<String>,
    /// Set accessibility page enabled. Explicit true or false.
    #[arg(long = "accessibility-page-enabled")]
    accessibility_page_enabled: Option<bool>,
    /// Set accessibility title.
    #[arg(long = "accessibility-title")]
    accessibility_title: Option<String>,
    /// Set accessibility content.
    #[arg(long = "accessibility-content")]
    accessibility_content: Option<String>,
    /// Set support popup text.
    #[arg(long = "support-popup-text")]
    support_popup_text: Option<String>,
    /// Set support page enabled. Explicit true or false.
    #[arg(long = "support-page-enabled")]
    support_page_enabled: Option<bool>,
    /// Set support page public. Explicit true or false.
    #[arg(long = "support-page-public")]
    support_page_public: Option<bool>,
    /// Set support title.
    #[arg(long = "support-title")]
    support_title: Option<String>,
    /// Set support page text.
    #[arg(long = "support-page-text")]
    support_page_text: Option<String>,
}
pub(crate) async fn execute(
    command: SettingsCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
) -> Result<CommandSuccess, AppError> {
    match command {
        SettingsCommand::Get => {
            let context = AuthenticatedContext::load(factory, store)?;
            let document = context
                .client()
                .global_settings(context.record().token())
                .await
                .map_err(|e| {
                    map_client_error(
                        e,
                        &Redactor::with_secret(context.record().token()),
                        "admin settings get",
                        false,
                    )
                })?;
            Ok(CommandSuccess::GlobalSettings(Box::new(
                GlobalSettingsSuccess {
                    settings: document.into(),
                },
            )))
        }
        SettingsCommand::Update(args) => {
            let request = if let Some(request) = args.source.read::<UpdateGlobalSettings>().await? {
                request
            } else {
                UpdateGlobalSettings {
                    disable_registration: args.disable_registration.map(Some),
                    disable_forgot_password: args.disable_forgot_password.map(Some),
                    product_name: args.product_name.map(Some),
                    display_authentication_method: args.display_authentication_method.map(Some),
                    default_authentication_method: args.default_authentication_method,
                    spinner_name: args.spinner_name.map(Some),
                    hide_logo: args.hide_logo.map(Some),
                    hide_card_counter_list: args.hide_card_counter_list.map(Some),
                    hide_board_member_list: args.hide_board_member_list.map(Some),
                    custom_login_logo_image_url: args.custom_login_logo_image_url.map(Some),
                    custom_login_logo_link_url: args.custom_login_logo_link_url.map(Some),
                    custom_help_link_url: args.custom_help_link_url.map(Some),
                    text_below_custom_login_logo: args.text_below_custom_login_logo.map(Some),
                    automatic_linked_url_schemes: args.automatic_linked_url_schemes.map(Some),
                    custom_top_left_corner_logo_image_url: args
                        .custom_top_left_corner_logo_image_url
                        .map(Some),
                    custom_top_left_corner_logo_link_url: args
                        .custom_top_left_corner_logo_link_url
                        .map(Some),
                    custom_top_left_corner_logo_height: args
                        .custom_top_left_corner_logo_height
                        .map(Some),
                    oidc_btn_text: args.oidc_btn_text.map(Some),
                    mail_domain_name: args.mail_domain_name.map(Some),
                    legal_notice: args.legal_notice.map(Some),
                    custom_head_enabled: args.custom_head_enabled.map(Some),
                    custom_head_meta_tags: args.custom_head_meta_tags.map(Some),
                    custom_head_link_tags: args.custom_head_link_tags.map(Some),
                    custom_manifest_enabled: args.custom_manifest_enabled.map(Some),
                    custom_manifest_content: args.custom_manifest_content.map(Some),
                    custom_asset_links_enabled: args.custom_asset_links_enabled.map(Some),
                    custom_asset_links_content: args.custom_asset_links_content.map(Some),
                    accessibility_page_enabled: args.accessibility_page_enabled.map(Some),
                    accessibility_title: args.accessibility_title.map(Some),
                    accessibility_content: args.accessibility_content.map(Some),
                    support_popup_text: args.support_popup_text.map(Some),
                    support_page_enabled: args.support_page_enabled.map(Some),
                    support_page_public: args.support_page_public.map(Some),
                    support_title: args.support_title.map(Some),
                    support_page_text: args.support_page_text.map(Some),
                }
            };
            let submitted_fields = submitted_fields(&request);
            input::validate_fields(&submitted_fields)?;
            let request = PreparedSettingsUpdate::new(&request)
                .map_err(|error| AppError::invalid_input(error.to_string()))?;
            let context = AuthenticatedContext::load(factory, store)?;
            let document = context
                .client()
                .update_global_settings(request, context.record().token())
                .await
                .map_err(|e| {
                    map_partial_mutation_client_error_with_not_found(
                        e,
                        &Redactor::with_secret(context.record().token()),
                        "admin settings update",
                        "the Wekan settings document was not found",
                    )
                })?;
            Ok(CommandSuccess::GlobalSettingsUpdated(Box::new(
                GlobalSettingsUpdateSuccess {
                    settings: document.into(),
                    submitted_fields,
                },
            )))
        }
    }
}

fn submitted_fields(request: &UpdateGlobalSettings) -> Vec<GlobalSettingField> {
    let mut fields = Vec::new();
    if request.disable_registration.is_some() {
        fields.push(GlobalSettingField::DisableRegistration);
    }
    if request.disable_forgot_password.is_some() {
        fields.push(GlobalSettingField::DisableForgotPassword);
    }
    if request.product_name.is_some() {
        fields.push(GlobalSettingField::ProductName);
    }
    if request.display_authentication_method.is_some() {
        fields.push(GlobalSettingField::DisplayAuthenticationMethod);
    }
    if request.default_authentication_method.is_some() {
        fields.push(GlobalSettingField::DefaultAuthenticationMethod);
    }
    if request.spinner_name.is_some() {
        fields.push(GlobalSettingField::SpinnerName);
    }
    if request.hide_logo.is_some() {
        fields.push(GlobalSettingField::HideLogo);
    }
    if request.hide_card_counter_list.is_some() {
        fields.push(GlobalSettingField::HideCardCounterList);
    }
    if request.hide_board_member_list.is_some() {
        fields.push(GlobalSettingField::HideBoardMemberList);
    }
    if request.custom_login_logo_image_url.is_some() {
        fields.push(GlobalSettingField::CustomLoginLogoImageUrl);
    }
    if request.custom_login_logo_link_url.is_some() {
        fields.push(GlobalSettingField::CustomLoginLogoLinkUrl);
    }
    if request.custom_help_link_url.is_some() {
        fields.push(GlobalSettingField::CustomHelpLinkUrl);
    }
    if request.text_below_custom_login_logo.is_some() {
        fields.push(GlobalSettingField::TextBelowCustomLoginLogo);
    }
    if request.automatic_linked_url_schemes.is_some() {
        fields.push(GlobalSettingField::AutomaticLinkedUrlSchemes);
    }
    if request.custom_top_left_corner_logo_image_url.is_some() {
        fields.push(GlobalSettingField::CustomTopLeftCornerLogoImageUrl);
    }
    if request.custom_top_left_corner_logo_link_url.is_some() {
        fields.push(GlobalSettingField::CustomTopLeftCornerLogoLinkUrl);
    }
    if request.custom_top_left_corner_logo_height.is_some() {
        fields.push(GlobalSettingField::CustomTopLeftCornerLogoHeight);
    }
    if request.oidc_btn_text.is_some() {
        fields.push(GlobalSettingField::OidcBtnText);
    }
    if request.mail_domain_name.is_some() {
        fields.push(GlobalSettingField::MailDomainName);
    }
    if request.legal_notice.is_some() {
        fields.push(GlobalSettingField::LegalNotice);
    }
    if request.custom_head_enabled.is_some() {
        fields.push(GlobalSettingField::CustomHeadEnabled);
    }
    if request.custom_head_meta_tags.is_some() {
        fields.push(GlobalSettingField::CustomHeadMetaTags);
    }
    if request.custom_head_link_tags.is_some() {
        fields.push(GlobalSettingField::CustomHeadLinkTags);
    }
    if request.custom_manifest_enabled.is_some() {
        fields.push(GlobalSettingField::CustomManifestEnabled);
    }
    if request.custom_manifest_content.is_some() {
        fields.push(GlobalSettingField::CustomManifestContent);
    }
    if request.custom_asset_links_enabled.is_some() {
        fields.push(GlobalSettingField::CustomAssetLinksEnabled);
    }
    if request.custom_asset_links_content.is_some() {
        fields.push(GlobalSettingField::CustomAssetLinksContent);
    }
    if request.accessibility_page_enabled.is_some() {
        fields.push(GlobalSettingField::AccessibilityPageEnabled);
    }
    if request.accessibility_title.is_some() {
        fields.push(GlobalSettingField::AccessibilityTitle);
    }
    if request.accessibility_content.is_some() {
        fields.push(GlobalSettingField::AccessibilityContent);
    }
    if request.support_popup_text.is_some() {
        fields.push(GlobalSettingField::SupportPopupText);
    }
    if request.support_page_enabled.is_some() {
        fields.push(GlobalSettingField::SupportPageEnabled);
    }
    if request.support_page_public.is_some() {
        fields.push(GlobalSettingField::SupportPagePublic);
    }
    if request.support_title.is_some() {
        fields.push(GlobalSettingField::SupportTitle);
    }
    if request.support_page_text.is_some() {
        fields.push(GlobalSettingField::SupportPageText);
    }
    fields
}
