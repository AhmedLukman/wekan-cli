use crate::client::admin::{
    BatchSize, CpuThreshold, DelayMs, NonNegativeNumber, SettingNumber,
    attachment_settings as storage, settings as global,
};
use serde::Serialize;

mod login_providers;
mod problems;
pub use login_providers::{
    BooleanConfigValue, ConfigSource, OauthProviderSettings, OauthProviderSource,
    OauthProviderSources, PasswordlessSettings, SecretConfigSource,
};
pub use problems::{
    DetectedProblem, EventActor, EventLocation, NewProblemCount, ProblemAcknowledgment,
    ProblemEvent, ProblemPage, ProblemsOverview, ProgressProblem,
};

use crate::client::admin::people::{
    DomainCount, FeatureField, OrganizationDocument, OrganizationSummary, TeamDocument, TeamSummary,
};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AdminDomainCollectionSuccess {
    pub domains: Vec<DomainCount>,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AdminOrganizationCollectionSuccess {
    pub organizations: Vec<OrganizationSummary>,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AdminTeamCollectionSuccess {
    pub teams: Vec<TeamSummary>,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AdminOrganizationUpdateSuccess {
    pub organization: OrganizationDocument,
    pub submitted_fields: Vec<FeatureField>,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AdminTeamUpdateSuccess {
    pub team: TeamDocument,
    pub submitted_fields: Vec<FeatureField>,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AdminFeaturesUpdateAllSuccess {
    pub field: FeatureField,
    pub value: bool,
    pub updated: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GlobalSettingField {
    DisableRegistration,
    DisableForgotPassword,
    ProductName,
    DisplayAuthenticationMethod,
    DefaultAuthenticationMethod,
    SpinnerName,
    HideLogo,
    HideCardCounterList,
    HideBoardMemberList,
    CustomLoginLogoImageUrl,
    CustomLoginLogoLinkUrl,
    CustomHelpLinkUrl,
    TextBelowCustomLoginLogo,
    AutomaticLinkedUrlSchemes,
    CustomTopLeftCornerLogoImageUrl,
    CustomTopLeftCornerLogoLinkUrl,
    CustomTopLeftCornerLogoHeight,
    OidcBtnText,
    MailDomainName,
    LegalNotice,
    CustomHeadEnabled,
    CustomHeadMetaTags,
    CustomHeadLinkTags,
    CustomManifestEnabled,
    CustomManifestContent,
    CustomAssetLinksEnabled,
    CustomAssetLinksContent,
    AccessibilityPageEnabled,
    AccessibilityTitle,
    AccessibilityContent,
    SupportPopupText,
    SupportPageEnabled,
    SupportPagePublic,
    SupportTitle,
    SupportPageText,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentSettingField {
    DefaultStorage,
    StorageConfig,
    UploadSettings,
    LimitSettings,
    MigrationSettings,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GlobalSettingsDetail {
    pub settings_id: Option<String>,
    pub disable_registration: Option<bool>,
    pub disable_forgot_password: Option<bool>,
    pub product_name: Option<String>,
    pub display_authentication_method: Option<bool>,
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
impl From<global::GlobalSettingsDocument> for GlobalSettingsDetail {
    fn from(d: global::GlobalSettingsDocument) -> Self {
        Self {
            settings_id: d.settings_id,
            disable_registration: d.disable_registration,
            disable_forgot_password: d.disable_forgot_password,
            product_name: d.product_name,
            display_authentication_method: d.display_authentication_method,
            default_authentication_method: d.default_authentication_method,
            spinner_name: d.spinner_name,
            hide_logo: d.hide_logo,
            hide_card_counter_list: d.hide_card_counter_list,
            hide_board_member_list: d.hide_board_member_list,
            custom_login_logo_image_url: d.custom_login_logo_image_url,
            custom_login_logo_link_url: d.custom_login_logo_link_url,
            custom_help_link_url: d.custom_help_link_url,
            text_below_custom_login_logo: d.text_below_custom_login_logo,
            automatic_linked_url_schemes: d.automatic_linked_url_schemes,
            custom_top_left_corner_logo_image_url: d.custom_top_left_corner_logo_image_url,
            custom_top_left_corner_logo_link_url: d.custom_top_left_corner_logo_link_url,
            custom_top_left_corner_logo_height: d.custom_top_left_corner_logo_height,
            oidc_btn_text: d.oidc_btn_text,
            mail_domain_name: d.mail_domain_name,
            legal_notice: d.legal_notice,
            custom_head_enabled: d.custom_head_enabled,
            custom_head_meta_tags: d.custom_head_meta_tags,
            custom_head_link_tags: d.custom_head_link_tags,
            custom_manifest_enabled: d.custom_manifest_enabled,
            custom_manifest_content: d.custom_manifest_content,
            custom_asset_links_enabled: d.custom_asset_links_enabled,
            custom_asset_links_content: d.custom_asset_links_content,
            accessibility_page_enabled: d.accessibility_page_enabled,
            accessibility_title: d.accessibility_title,
            accessibility_content: d.accessibility_content,
            support_popup_text: d.support_popup_text,
            support_page_enabled: d.support_page_enabled,
            support_page_public: d.support_page_public,
            support_title: d.support_title,
            support_page_text: d.support_page_text,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FilesystemSettingsDetail {
    pub enabled: bool,
    pub read: bool,
    pub write: bool,
    pub path: Option<String>,
}
impl From<storage::FilesystemDocument> for FilesystemSettingsDetail {
    fn from(d: storage::FilesystemDocument) -> Self {
        Self {
            enabled: d.enabled,
            read: d.read,
            write: d.write,
            path: d.path,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GridfsSettingsDetail {
    pub enabled: bool,
    pub read: bool,
    pub write: bool,
}
impl From<storage::GridfsDocument> for GridfsSettingsDetail {
    fn from(d: storage::GridfsDocument) -> Self {
        Self {
            enabled: d.enabled,
            read: d.read,
            write: d.write,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct S3SettingsDetail {
    pub enabled: Option<bool>,
    pub read: Option<bool>,
    pub write: Option<bool>,
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub bucket: Option<String>,
    pub access_key_id: Option<String>,
    pub force_path_style: Option<bool>,
    pub secret_access_key_set: bool,
}
impl From<storage::S3Document> for S3SettingsDetail {
    fn from(d: storage::S3Document) -> Self {
        Self {
            enabled: d.enabled,
            read: d.read,
            write: d.write,
            endpoint: d.endpoint,
            region: d.region,
            bucket: d.bucket,
            access_key_id: d.access_key_id,
            force_path_style: d.force_path_style,
            secret_access_key_set: d.secret_access_key_set,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AzureSettingsDetail {
    pub enabled: Option<bool>,
    pub read: Option<bool>,
    pub write: Option<bool>,
    pub account_name: Option<String>,
    pub bucket: Option<String>,
    pub account_key_set: bool,
    pub connection_string_set: bool,
}
impl From<storage::AzureDocument> for AzureSettingsDetail {
    fn from(d: storage::AzureDocument) -> Self {
        Self {
            enabled: d.enabled,
            read: d.read,
            write: d.write,
            account_name: d.account_name,
            bucket: d.bucket,
            account_key_set: d.account_key_set,
            connection_string_set: d.connection_string_set,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GcsSettingsDetail {
    pub enabled: Option<bool>,
    pub read: Option<bool>,
    pub write: Option<bool>,
    pub project_id: Option<String>,
    pub key_filename: Option<String>,
    pub bucket: Option<String>,
    pub credentials_set: bool,
}
impl From<storage::GcsDocument> for GcsSettingsDetail {
    fn from(d: storage::GcsDocument) -> Self {
        Self {
            enabled: d.enabled,
            read: d.read,
            write: d.write,
            project_id: d.project_id,
            key_filename: d.key_filename,
            bucket: d.bucket,
            credentials_set: d.credentials_set,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct UploadSettingsDetail {
    pub max_file_size: Option<SettingNumber>,
    pub allowed_mime_types: Option<Vec<String>>,
}
impl From<storage::UploadDocument> for UploadSettingsDetail {
    fn from(d: storage::UploadDocument) -> Self {
        Self {
            max_file_size: d.max_file_size,
            allowed_mime_types: d.allowed_mime_types,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LimitsSettingsDetail {
    pub attachments_upload_max_bytes: Option<NonNegativeNumber>,
    pub attachments_upload_blocked: Option<bool>,
    pub avatars_upload_blocked: Option<bool>,
    pub attachments_download_max_bytes: Option<NonNegativeNumber>,
    pub attachments_download_blocked: Option<bool>,
    pub api_upload_max_bytes: Option<NonNegativeNumber>,
    pub api_upload_blocked: Option<bool>,
    pub api_download_max_bytes: Option<NonNegativeNumber>,
    pub api_download_blocked: Option<bool>,
}
impl From<storage::LimitsDocument> for LimitsSettingsDetail {
    fn from(d: storage::LimitsDocument) -> Self {
        Self {
            attachments_upload_max_bytes: d.attachments_upload_max_bytes,
            attachments_upload_blocked: d.attachments_upload_blocked,
            avatars_upload_blocked: d.avatars_upload_blocked,
            attachments_download_max_bytes: d.attachments_download_max_bytes,
            attachments_download_blocked: d.attachments_download_blocked,
            api_upload_max_bytes: d.api_upload_max_bytes,
            api_upload_blocked: d.api_upload_blocked,
            api_download_max_bytes: d.api_download_max_bytes,
            api_download_blocked: d.api_download_blocked,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MigrationSettingsDetail {
    pub auto_migrate: bool,
    pub batch_size: BatchSize,
    pub delay_ms: DelayMs,
    pub cpu_threshold: CpuThreshold,
}
impl From<storage::MigrationDocument> for MigrationSettingsDetail {
    fn from(d: storage::MigrationDocument) -> Self {
        Self {
            auto_migrate: d.auto_migrate,
            batch_size: d.batch_size,
            delay_ms: d.delay_ms,
            cpu_threshold: d.cpu_threshold,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WebdavSettingsDetail {
    pub enabled: Option<bool>,
    pub read: Option<bool>,
    pub write: Option<bool>,
    pub endpoint: Option<String>,
    pub username: Option<String>,
    pub bucket: Option<String>,
    pub password_set: bool,
}
impl From<storage::WebdavDocument> for WebdavSettingsDetail {
    fn from(d: storage::WebdavDocument) -> Self {
        Self {
            enabled: d.enabled,
            read: d.read,
            write: d.write,
            endpoint: d.endpoint,
            username: d.username,
            bucket: d.bucket,
            password_set: d.password_set,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StorageConfigDetail {
    pub webdav: Option<WebdavSettingsDetail>,
    pub filesystem: Option<FilesystemSettingsDetail>,
    pub gridfs: Option<GridfsSettingsDetail>,
    pub s3: Option<S3SettingsDetail>,
    pub azure: Option<AzureSettingsDetail>,
    pub gcs: Option<GcsSettingsDetail>,
}
impl From<storage::StorageConfigDocument> for StorageConfigDetail {
    fn from(d: storage::StorageConfigDocument) -> Self {
        Self {
            webdav: d.webdav.map(Into::into),
            filesystem: d.filesystem.map(Into::into),
            gridfs: d.gridfs.map(Into::into),
            s3: d.s3.map(Into::into),
            azure: d.azure.map(Into::into),
            gcs: d.gcs.map(Into::into),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AttachmentSettingsDetail {
    pub default_storage: storage::StorageBackend,
    pub storage_config: Option<StorageConfigDetail>,
    pub upload_settings: Option<UploadSettingsDetail>,
    pub limit_settings: Option<LimitsSettingsDetail>,
    pub migration_settings: Option<MigrationSettingsDetail>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub settings_id: String,
}
impl From<storage::AttachmentSettingsDocument> for AttachmentSettingsDetail {
    fn from(d: storage::AttachmentSettingsDocument) -> Self {
        Self {
            default_storage: d.default_storage,
            storage_config: d.storage_config.map(Into::into),
            upload_settings: d.upload_settings.map(Into::into),
            limit_settings: d.limit_settings.map(Into::into),
            migration_settings: d.migration_settings.map(Into::into),
            created_at: d.created_at,
            updated_at: d.updated_at,
            created_by: d.created_by,
            updated_by: d.updated_by,
            settings_id: d.settings_id,
        }
    }
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GlobalSettingsSuccess {
    pub settings: GlobalSettingsDetail,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct GlobalSettingsUpdateSuccess {
    pub settings: GlobalSettingsDetail,
    pub submitted_fields: Vec<GlobalSettingField>,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AttachmentSettingsSuccess {
    pub settings: AttachmentSettingsDetail,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct AttachmentSettingsUpdateSuccess {
    pub settings: AttachmentSettingsDetail,
    pub submitted_fields: Vec<AttachmentSettingField>,
}
