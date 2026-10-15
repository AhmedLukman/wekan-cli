use super::{
    BatchSize, CpuThreshold, DelayMs, GcsCredentials, MaskedSecret, NonNegativeNumber,
    SettingNumber, StorageSecret, nullable_object, present, present_object,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FilesystemDocument {
    pub enabled: bool,
    pub read: bool,
    pub write: bool,
    #[serde(default, deserialize_with = "present")]
    pub path: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FilesystemUpdate {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub read: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub write: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GridfsDocument {
    pub enabled: bool,
    pub read: bool,
    pub write: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GridfsUpdate {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub read: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub write: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct S3Document {
    #[serde(default, deserialize_with = "present")]
    pub enabled: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub read: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub write: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub endpoint: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub region: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub bucket: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub access_key_id: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub force_path_style: Option<bool>,
    pub secret_access_key: MaskedSecret,
    pub secret_access_key_set: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct S3Update {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub read: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub write: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub endpoint: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub region: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub bucket: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub access_key_id: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub force_path_style: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub secret_access_key: Option<StorageSecret>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AzureDocument {
    #[serde(default, deserialize_with = "present")]
    pub enabled: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub read: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub write: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub account_name: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub bucket: Option<String>,
    pub account_key: MaskedSecret,
    pub account_key_set: bool,
    pub connection_string: MaskedSecret,
    pub connection_string_set: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AzureUpdate {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub read: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub write: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub account_name: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub bucket: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub account_key: Option<StorageSecret>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub connection_string: Option<StorageSecret>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GcsDocument {
    #[serde(default, deserialize_with = "present")]
    pub enabled: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub read: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub write: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub project_id: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub key_filename: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub bucket: Option<String>,
    pub credentials: MaskedSecret,
    pub credentials_set: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GcsUpdate {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub read: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub write: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub project_id: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub key_filename: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub bucket: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub credentials: Option<GcsCredentials>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UploadDocument {
    #[serde(default, deserialize_with = "present")]
    pub max_file_size: Option<SettingNumber>,
    #[serde(default, deserialize_with = "present")]
    pub allowed_mime_types: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UploadUpdate {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_file_size: Option<SettingNumber>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub allowed_mime_types: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LimitsDocument {
    #[serde(default, deserialize_with = "present")]
    pub attachments_upload_max_bytes: Option<NonNegativeNumber>,
    #[serde(default, deserialize_with = "present")]
    pub attachments_upload_blocked: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub avatars_upload_blocked: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub attachments_download_max_bytes: Option<NonNegativeNumber>,
    #[serde(default, deserialize_with = "present")]
    pub attachments_download_blocked: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub api_upload_max_bytes: Option<NonNegativeNumber>,
    #[serde(default, deserialize_with = "present")]
    pub api_upload_blocked: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    pub api_download_max_bytes: Option<NonNegativeNumber>,
    #[serde(default, deserialize_with = "present")]
    pub api_download_blocked: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LimitsUpdate {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub attachments_upload_max_bytes: Option<NonNegativeNumber>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub attachments_upload_blocked: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub avatars_upload_blocked: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub attachments_download_max_bytes: Option<NonNegativeNumber>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub attachments_download_blocked: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub api_upload_max_bytes: Option<NonNegativeNumber>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub api_upload_blocked: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub api_download_max_bytes: Option<NonNegativeNumber>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub api_download_blocked: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MigrationDocument {
    pub auto_migrate: bool,
    pub batch_size: BatchSize,
    pub delay_ms: DelayMs,
    pub cpu_threshold: CpuThreshold,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MigrationUpdate {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub auto_migrate: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub batch_size: Option<BatchSize>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub delay_ms: Option<DelayMs>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub cpu_threshold: Option<CpuThreshold>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageConfigDocument {
    #[serde(default, deserialize_with = "present_object")]
    pub filesystem: Option<FilesystemDocument>,
    #[serde(default, deserialize_with = "present_object")]
    pub gridfs: Option<GridfsDocument>,
    #[serde(default, deserialize_with = "present_object")]
    pub s3: Option<S3Document>,
    #[serde(default, deserialize_with = "present_object")]
    pub azure: Option<AzureDocument>,
    #[serde(default, deserialize_with = "present_object")]
    pub gcs: Option<GcsDocument>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StorageConfigUpdate {
    #[serde(
        default,
        deserialize_with = "present_object",
        skip_serializing_if = "Option::is_none"
    )]
    pub filesystem: Option<FilesystemUpdate>,
    #[serde(
        default,
        deserialize_with = "present_object",
        skip_serializing_if = "Option::is_none"
    )]
    pub gridfs: Option<GridfsUpdate>,
    #[serde(
        default,
        deserialize_with = "present_object",
        skip_serializing_if = "Option::is_none"
    )]
    pub s3: Option<S3Update>,
    #[serde(
        default,
        deserialize_with = "present_object",
        skip_serializing_if = "Option::is_none"
    )]
    pub azure: Option<AzureUpdate>,
    #[serde(
        default,
        deserialize_with = "present_object",
        skip_serializing_if = "Option::is_none"
    )]
    pub gcs: Option<GcsUpdate>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackend {
    #[default]
    Fs,
    Gridfs,
    S3,
    Azure,
    Gcs,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AttachmentSettingsDocument {
    pub default_storage: StorageBackend,
    #[serde(default, deserialize_with = "nullable_object")]
    pub storage_config: Option<StorageConfigDocument>,
    #[serde(default, deserialize_with = "nullable_object")]
    pub upload_settings: Option<UploadDocument>,
    #[serde(default, deserialize_with = "nullable_object")]
    pub limit_settings: Option<LimitsDocument>,
    #[serde(default, deserialize_with = "nullable_object")]
    pub migration_settings: Option<MigrationDocument>,
    #[serde(deserialize_with = "super::date")]
    pub created_at: String,
    #[serde(deserialize_with = "super::date")]
    pub updated_at: String,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub updated_by: Option<String>,
    #[serde(rename = "_id")]
    pub settings_id: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateAttachmentSettings {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub default_storage: Option<StorageBackend>,
    #[serde(
        default,
        deserialize_with = "present_object",
        skip_serializing_if = "Option::is_none"
    )]
    pub storage_config: Option<StorageConfigUpdate>,
    #[serde(
        default,
        deserialize_with = "present_object",
        skip_serializing_if = "Option::is_none"
    )]
    pub upload_settings: Option<UploadUpdate>,
    #[serde(
        default,
        deserialize_with = "present_object",
        skip_serializing_if = "Option::is_none"
    )]
    pub limit_settings: Option<LimitsUpdate>,
    #[serde(
        default,
        deserialize_with = "present_object",
        skip_serializing_if = "Option::is_none"
    )]
    pub migration_settings: Option<MigrationUpdate>,
}

impl std::str::FromStr for StorageBackend {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "fs" => Ok(Self::Fs),
            "gridfs" => Ok(Self::Gridfs),
            "s3" => Ok(Self::S3),
            "azure" => Ok(Self::Azure),
            "gcs" => Ok(Self::Gcs),
            _ => Err("expected fs, gridfs, s3, azure, or gcs"),
        }
    }
}

impl UpdateAttachmentSettings {
    pub(crate) fn redaction_secrets(&self) -> Vec<secrecy::SecretString> {
        let mut secrets = Vec::new();
        if let Some(config) = &self.storage_config {
            if let Some(provider) = &config.s3 {
                if let Some(secret) = &provider.secret_access_key {
                    secrets.push(secret.0.clone());
                }
            }
            if let Some(provider) = &config.azure {
                if let Some(secret) = &provider.account_key {
                    secrets.push(secret.0.clone());
                }
                if let Some(secret) = &provider.connection_string {
                    secrets.push(secret.0.clone());
                }
            }
            if let Some(provider) = &config.gcs {
                if let Some(secret) = &provider.credentials {
                    secrets.extend(secret.redaction_secrets());
                }
            }
        }
        secrets
    }
}
