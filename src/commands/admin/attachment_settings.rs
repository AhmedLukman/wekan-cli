use super::input;
use crate::client::admin::{
    BatchSize, CpuThreshold, DelayMs, GcsCredentials, NonNegativeNumber, PreparedSettingsUpdate,
    SettingNumber, StorageSecret, attachment_settings::*,
};
use crate::{
    client::WekanClientFactory,
    command_result::*,
    commands::{
        authenticated::AuthenticatedContext,
        client_error::{map_client_error, map_partial_mutation_client_error_with_not_found},
    },
    credentials::{CredentialStore, SecretInputProvider},
    error::AppError,
    redaction::Redactor,
};
use clap::{ArgGroup, Args, Subcommand};
#[derive(Debug, Args)]
pub struct AttachmentSettingsArgs {
    #[command(subcommand)]
    pub command: AttachmentSettingsCommand,
}
#[derive(Debug, Subcommand)]
pub enum AttachmentSettingsCommand {
    /// Return the complete mapped settings document.
    Get,
    /// Apply native settings updates, with no read-before-write merging.
    Update(Box<UpdateArgs>),
}
#[derive(Debug, Args)]
#[command(group(ArgGroup::new("credential_stdin").args([
    "s3_secret_access_key_stdin",
    "azure_account_key_stdin",
    "azure_connection_string_stdin",
    "gcs_credentials_stdin",
]).multiple(false)), group(ArgGroup::new("setting_flags").args([
    "default_storage",
    "filesystem_enabled",
    "filesystem_read",
    "filesystem_write",
    "filesystem_path",
    "gridfs_enabled",
    "gridfs_read",
    "gridfs_write",
    "s3_enabled",
    "s3_read",
    "s3_write",
    "s3_endpoint",
    "s3_region",
    "s3_bucket",
    "s3_access_key_id",
    "s3_force_path_style",
    "azure_enabled",
    "azure_read",
    "azure_write",
    "azure_account_name",
    "azure_bucket",
    "gcs_enabled",
    "gcs_read",
    "gcs_write",
    "gcs_project_id",
    "gcs_key_filename",
    "gcs_bucket",
    "upload_max_file_size",
    "upload_allowed_mime_types",
    "attachments_upload_max_bytes",
    "attachments_upload_blocked",
    "avatars_upload_blocked",
    "attachments_download_max_bytes",
    "attachments_download_blocked",
    "api_upload_max_bytes",
    "api_upload_blocked",
    "api_download_max_bytes",
    "api_download_blocked",
    "migration_auto_migrate",
    "migration_batch_size",
    "migration_delay_ms",
    "migration_cpu_threshold",
    "s3_secret_access_key_file",
    "s3_secret_access_key_stdin",
    "azure_account_key_file",
    "azure_account_key_stdin",
    "azure_connection_string_file",
    "azure_connection_string_stdin",
    "gcs_credentials_file",
    "gcs_credentials_stdin",
    "clear_upload_allowed_mime_types",
]).multiple(true).conflicts_with("settings_source")), group(ArgGroup::new("update_input").args([
    "settings_file",
    "settings_stdin",
    "default_storage",
    "filesystem_enabled",
    "filesystem_read",
    "filesystem_write",
    "filesystem_path",
    "gridfs_enabled",
    "gridfs_read",
    "gridfs_write",
    "s3_enabled",
    "s3_read",
    "s3_write",
    "s3_endpoint",
    "s3_region",
    "s3_bucket",
    "s3_access_key_id",
    "s3_force_path_style",
    "azure_enabled",
    "azure_read",
    "azure_write",
    "azure_account_name",
    "azure_bucket",
    "gcs_enabled",
    "gcs_read",
    "gcs_write",
    "gcs_project_id",
    "gcs_key_filename",
    "gcs_bucket",
    "upload_max_file_size",
    "upload_allowed_mime_types",
    "attachments_upload_max_bytes",
    "attachments_upload_blocked",
    "avatars_upload_blocked",
    "attachments_download_max_bytes",
    "attachments_download_blocked",
    "api_upload_max_bytes",
    "api_upload_blocked",
    "api_download_max_bytes",
    "api_download_blocked",
    "migration_auto_migrate",
    "migration_batch_size",
    "migration_delay_ms",
    "migration_cpu_threshold",
    "s3_secret_access_key_file",
    "s3_secret_access_key_stdin",
    "azure_account_key_file",
    "azure_account_key_stdin",
    "azure_connection_string_file",
    "azure_connection_string_stdin",
    "gcs_credentials_file",
    "gcs_credentials_stdin",
    "clear_upload_allowed_mime_types",
]).required(true).multiple(true)), after_help = "Nested sections use Wekan replacement/default semantics: omitted siblings can reset or disappear. Cloud providers merge and blank secrets preserve stored credentials. Saving does not verify connectivity. JSON sources conflict with every setting flag.")]
pub struct UpdateArgs {
    #[command(flatten)]
    source: input::SourceArgs,
    /// Default backend: fs, gridfs, s3, azure, or gcs.
    #[arg(long = "default-storage")]
    default_storage: Option<StorageBackend>,
    /// Set filesystem enabled. Explicit true or false.
    #[arg(long = "filesystem-enabled")]
    filesystem_enabled: Option<bool>,
    /// Set filesystem read. Explicit true or false.
    #[arg(long = "filesystem-read")]
    filesystem_read: Option<bool>,
    /// Set filesystem write. Explicit true or false.
    #[arg(long = "filesystem-write")]
    filesystem_write: Option<bool>,
    /// Set filesystem path.
    #[arg(long = "filesystem-path")]
    filesystem_path: Option<String>,
    /// Set gridfs enabled. Explicit true or false.
    #[arg(long = "gridfs-enabled")]
    gridfs_enabled: Option<bool>,
    /// Set gridfs read. Explicit true or false.
    #[arg(long = "gridfs-read")]
    gridfs_read: Option<bool>,
    /// Set gridfs write. Explicit true or false.
    #[arg(long = "gridfs-write")]
    gridfs_write: Option<bool>,
    /// Set s3 enabled. Explicit true or false.
    #[arg(long = "s3-enabled")]
    s3_enabled: Option<bool>,
    /// Set s3 read. Explicit true or false.
    #[arg(long = "s3-read")]
    s3_read: Option<bool>,
    /// Set s3 write. Explicit true or false.
    #[arg(long = "s3-write")]
    s3_write: Option<bool>,
    /// Set s3 endpoint.
    #[arg(long = "s3-endpoint")]
    s3_endpoint: Option<String>,
    /// Set s3 region.
    #[arg(long = "s3-region")]
    s3_region: Option<String>,
    /// Set s3 bucket.
    #[arg(long = "s3-bucket")]
    s3_bucket: Option<String>,
    /// Set s3 access key id.
    #[arg(long = "s3-access-key-id")]
    s3_access_key_id: Option<String>,
    /// Set s3 force path style. Explicit true or false.
    #[arg(long = "s3-force-path-style")]
    s3_force_path_style: Option<bool>,
    /// Set azure enabled. Explicit true or false.
    #[arg(long = "azure-enabled")]
    azure_enabled: Option<bool>,
    /// Set azure read. Explicit true or false.
    #[arg(long = "azure-read")]
    azure_read: Option<bool>,
    /// Set azure write. Explicit true or false.
    #[arg(long = "azure-write")]
    azure_write: Option<bool>,
    /// Set azure account name.
    #[arg(long = "azure-account-name")]
    azure_account_name: Option<String>,
    /// Set azure bucket.
    #[arg(long = "azure-bucket")]
    azure_bucket: Option<String>,
    /// Set gcs enabled. Explicit true or false.
    #[arg(long = "gcs-enabled")]
    gcs_enabled: Option<bool>,
    /// Set gcs read. Explicit true or false.
    #[arg(long = "gcs-read")]
    gcs_read: Option<bool>,
    /// Set gcs write. Explicit true or false.
    #[arg(long = "gcs-write")]
    gcs_write: Option<bool>,
    /// Set gcs project id.
    #[arg(long = "gcs-project-id")]
    gcs_project_id: Option<String>,
    /// Set gcs key filename.
    #[arg(long = "gcs-key-filename")]
    gcs_key_filename: Option<String>,
    /// Set gcs bucket.
    #[arg(long = "gcs-bucket")]
    gcs_bucket: Option<String>,
    /// Set upload max file size.
    #[arg(long = "upload-max-file-size", allow_hyphen_values = true)]
    upload_max_file_size: Option<SettingNumber>,
    /// Replace allowed MIME types; repeat for each entry.
    #[arg(
        long = "upload-allowed-mime-type",
        conflicts_with = "clear_upload_allowed_mime_types"
    )]
    upload_allowed_mime_types: Vec<String>,
    /// Set attachments upload max bytes.
    #[arg(long = "attachments-upload-max-bytes", allow_hyphen_values = true)]
    attachments_upload_max_bytes: Option<NonNegativeNumber>,
    /// Set attachments upload blocked. Explicit true or false.
    #[arg(long = "attachments-upload-blocked")]
    attachments_upload_blocked: Option<bool>,
    /// Set avatars upload blocked. Explicit true or false.
    #[arg(long = "avatars-upload-blocked")]
    avatars_upload_blocked: Option<bool>,
    /// Set attachments download max bytes.
    #[arg(long = "attachments-download-max-bytes", allow_hyphen_values = true)]
    attachments_download_max_bytes: Option<NonNegativeNumber>,
    /// Set attachments download blocked. Explicit true or false.
    #[arg(long = "attachments-download-blocked")]
    attachments_download_blocked: Option<bool>,
    /// Set api upload max bytes.
    #[arg(long = "api-upload-max-bytes", allow_hyphen_values = true)]
    api_upload_max_bytes: Option<NonNegativeNumber>,
    /// Set api upload blocked. Explicit true or false.
    #[arg(long = "api-upload-blocked")]
    api_upload_blocked: Option<bool>,
    /// Set api download max bytes.
    #[arg(long = "api-download-max-bytes", allow_hyphen_values = true)]
    api_download_max_bytes: Option<NonNegativeNumber>,
    /// Set api download blocked. Explicit true or false.
    #[arg(long = "api-download-blocked")]
    api_download_blocked: Option<bool>,
    /// Set migration auto migrate. Explicit true or false.
    #[arg(long = "migration-auto-migrate")]
    migration_auto_migrate: Option<bool>,
    /// Set migration batch size.
    #[arg(long = "migration-batch-size", allow_hyphen_values = true)]
    migration_batch_size: Option<BatchSize>,
    /// Set migration delay ms.
    #[arg(long = "migration-delay-ms", allow_hyphen_values = true)]
    migration_delay_ms: Option<DelayMs>,
    /// Set migration cpu threshold.
    #[arg(long = "migration-cpu-threshold", allow_hyphen_values = true)]
    migration_cpu_threshold: Option<CpuThreshold>,
    /// Read a storage credential from a regular UTF-8 file; blank preserves the stored secret.
    #[arg(
        long = "s3-secret-access-key-file",
        conflicts_with = "s3_secret_access_key_stdin",
        value_name = "PATH"
    )]
    s3_secret_access_key_file: Option<std::path::PathBuf>,
    /// Read a storage credential from UTF-8 stdin; blank preserves the stored secret.
    #[arg(
        long = "s3-secret-access-key-stdin",
        conflicts_with = "s3_secret_access_key_file"
    )]
    s3_secret_access_key_stdin: bool,
    /// Read a storage credential from a regular UTF-8 file; blank preserves the stored secret.
    #[arg(
        long = "azure-account-key-file",
        conflicts_with = "azure_account_key_stdin",
        value_name = "PATH"
    )]
    azure_account_key_file: Option<std::path::PathBuf>,
    /// Read a storage credential from UTF-8 stdin; blank preserves the stored secret.
    #[arg(
        long = "azure-account-key-stdin",
        conflicts_with = "azure_account_key_file"
    )]
    azure_account_key_stdin: bool,
    /// Read a storage credential from a regular UTF-8 file; blank preserves the stored secret.
    #[arg(
        long = "azure-connection-string-file",
        conflicts_with = "azure_connection_string_stdin",
        value_name = "PATH"
    )]
    azure_connection_string_file: Option<std::path::PathBuf>,
    /// Read a storage credential from UTF-8 stdin; blank preserves the stored secret.
    #[arg(
        long = "azure-connection-string-stdin",
        conflicts_with = "azure_connection_string_file"
    )]
    azure_connection_string_stdin: bool,
    /// Read a storage credential from a regular UTF-8 file; blank preserves the stored secret.
    #[arg(
        long = "gcs-credentials-file",
        conflicts_with = "gcs_credentials_stdin",
        value_name = "PATH"
    )]
    gcs_credentials_file: Option<std::path::PathBuf>,
    /// Read a storage credential from UTF-8 stdin; blank preserves the stored secret.
    #[arg(
        long = "gcs-credentials-stdin",
        conflicts_with = "gcs_credentials_file"
    )]
    gcs_credentials_stdin: bool,
    /// Replace the allowed MIME type list with an empty array.
    #[arg(
        long = "clear-upload-allowed-mime-types",
        conflicts_with = "upload_allowed_mime_types"
    )]
    clear_upload_allowed_mime_types: bool,
}
impl UpdateArgs {
    async fn request(
        self,
        secrets: &dyn SecretInputProvider,
    ) -> Result<UpdateAttachmentSettings, AppError> {
        if let Some(request) = self.source.read().await? {
            return Ok(request);
        }
        let s3_secret_access_key = input::secret(
            self.s3_secret_access_key_file,
            self.s3_secret_access_key_stdin,
            secrets,
        )
        .await?
        .map(StorageSecret);
        let azure_account_key = input::secret(
            self.azure_account_key_file,
            self.azure_account_key_stdin,
            secrets,
        )
        .await?
        .map(StorageSecret);
        let azure_connection_string = input::secret(
            self.azure_connection_string_file,
            self.azure_connection_string_stdin,
            secrets,
        )
        .await?
        .map(StorageSecret);
        let gcs_credentials = input::secret(
            self.gcs_credentials_file,
            self.gcs_credentials_stdin,
            secrets,
        )
        .await?
        .map(GcsCredentials::text);
        let filesystem = FilesystemUpdate {
            enabled: self.filesystem_enabled,
            read: self.filesystem_read,
            write: self.filesystem_write,
            path: self.filesystem_path,
        };
        let filesystem = if filesystem.enabled.is_some()
            || filesystem.read.is_some()
            || filesystem.write.is_some()
            || filesystem.path.is_some()
        {
            Some(filesystem)
        } else {
            None
        };
        let gridfs = GridfsUpdate {
            enabled: self.gridfs_enabled,
            read: self.gridfs_read,
            write: self.gridfs_write,
        };
        let gridfs = if gridfs.enabled.is_some() || gridfs.read.is_some() || gridfs.write.is_some()
        {
            Some(gridfs)
        } else {
            None
        };
        let s3 = S3Update {
            enabled: self.s3_enabled,
            read: self.s3_read,
            write: self.s3_write,
            endpoint: self.s3_endpoint,
            region: self.s3_region,
            bucket: self.s3_bucket,
            access_key_id: self.s3_access_key_id,
            force_path_style: self.s3_force_path_style,
            secret_access_key: s3_secret_access_key,
        };
        let s3 = if s3.enabled.is_some()
            || s3.read.is_some()
            || s3.write.is_some()
            || s3.endpoint.is_some()
            || s3.region.is_some()
            || s3.bucket.is_some()
            || s3.access_key_id.is_some()
            || s3.force_path_style.is_some()
            || s3.secret_access_key.is_some()
        {
            Some(s3)
        } else {
            None
        };
        let azure = AzureUpdate {
            enabled: self.azure_enabled,
            read: self.azure_read,
            write: self.azure_write,
            account_name: self.azure_account_name,
            bucket: self.azure_bucket,
            account_key: azure_account_key,
            connection_string: azure_connection_string,
        };
        let azure = if azure.enabled.is_some()
            || azure.read.is_some()
            || azure.write.is_some()
            || azure.account_name.is_some()
            || azure.bucket.is_some()
            || azure.account_key.is_some()
            || azure.connection_string.is_some()
        {
            Some(azure)
        } else {
            None
        };
        let gcs = GcsUpdate {
            enabled: self.gcs_enabled,
            read: self.gcs_read,
            write: self.gcs_write,
            project_id: self.gcs_project_id,
            key_filename: self.gcs_key_filename,
            bucket: self.gcs_bucket,
            credentials: gcs_credentials,
        };
        let gcs = if gcs.enabled.is_some()
            || gcs.read.is_some()
            || gcs.write.is_some()
            || gcs.project_id.is_some()
            || gcs.key_filename.is_some()
            || gcs.bucket.is_some()
            || gcs.credentials.is_some()
        {
            Some(gcs)
        } else {
            None
        };
        let upload = UploadUpdate {
            max_file_size: self.upload_max_file_size,
            allowed_mime_types: if self.clear_upload_allowed_mime_types {
                Some(Vec::new())
            } else if self.upload_allowed_mime_types.is_empty() {
                None
            } else {
                Some(self.upload_allowed_mime_types)
            },
        };
        let upload = if upload.max_file_size.is_some() || upload.allowed_mime_types.is_some() {
            Some(upload)
        } else {
            None
        };
        let limits = LimitsUpdate {
            attachments_upload_max_bytes: self.attachments_upload_max_bytes,
            attachments_upload_blocked: self.attachments_upload_blocked,
            avatars_upload_blocked: self.avatars_upload_blocked,
            attachments_download_max_bytes: self.attachments_download_max_bytes,
            attachments_download_blocked: self.attachments_download_blocked,
            api_upload_max_bytes: self.api_upload_max_bytes,
            api_upload_blocked: self.api_upload_blocked,
            api_download_max_bytes: self.api_download_max_bytes,
            api_download_blocked: self.api_download_blocked,
        };
        let limits = if limits.attachments_upload_max_bytes.is_some()
            || limits.attachments_upload_blocked.is_some()
            || limits.avatars_upload_blocked.is_some()
            || limits.attachments_download_max_bytes.is_some()
            || limits.attachments_download_blocked.is_some()
            || limits.api_upload_max_bytes.is_some()
            || limits.api_upload_blocked.is_some()
            || limits.api_download_max_bytes.is_some()
            || limits.api_download_blocked.is_some()
        {
            Some(limits)
        } else {
            None
        };
        let migration = MigrationUpdate {
            auto_migrate: self.migration_auto_migrate,
            batch_size: self.migration_batch_size,
            delay_ms: self.migration_delay_ms,
            cpu_threshold: self.migration_cpu_threshold,
        };
        let migration = if migration.auto_migrate.is_some()
            || migration.batch_size.is_some()
            || migration.delay_ms.is_some()
            || migration.cpu_threshold.is_some()
        {
            Some(migration)
        } else {
            None
        };
        let storage_config = StorageConfigUpdate {
            filesystem,
            gridfs,
            s3,
            azure,
            gcs,
        };
        let has_config = storage_config.filesystem.is_some()
            || storage_config.gridfs.is_some()
            || storage_config.s3.is_some()
            || storage_config.azure.is_some()
            || storage_config.gcs.is_some();
        Ok(UpdateAttachmentSettings {
            default_storage: self.default_storage,
            storage_config: has_config.then_some(storage_config),
            upload_settings: upload,
            limit_settings: limits,
            migration_settings: migration,
        })
    }
}
pub(crate) async fn execute(
    command: AttachmentSettingsCommand,
    factory: &WekanClientFactory,
    store: &dyn CredentialStore,
    secrets: &dyn SecretInputProvider,
) -> Result<CommandSuccess, AppError> {
    match command {
        AttachmentSettingsCommand::Get => {
            let context = AuthenticatedContext::load(factory, store)?;
            let document = context
                .client()
                .attachment_settings(context.record().token())
                .await
                .map_err(|e| {
                    map_client_error(
                        e,
                        &Redactor::with_secret(context.record().token()),
                        "admin attachment-settings get",
                        false,
                    )
                })?;
            Ok(CommandSuccess::AttachmentSettings(Box::new(
                AttachmentSettingsSuccess {
                    settings: document.into(),
                },
            )))
        }
        AttachmentSettingsCommand::Update(args) => {
            let request = args.request(secrets).await?;
            let submitted_fields = submitted_fields(&request);
            input::validate_fields(&submitted_fields)?;
            let prepared = PreparedSettingsUpdate::new(&request)
                .map_err(|error| AppError::invalid_input(error.to_string()))?;
            let context = AuthenticatedContext::load(factory, store)?;
            let secrets = request.redaction_secrets();
            let mut redactor = Redactor::with_secret(context.record().token());
            for secret in &secrets {
                redactor = redactor.and_secret(secret);
            }
            let document = context
                .client()
                .update_attachment_settings(prepared, context.record().token())
                .await
                .map_err(|e| {
                    map_partial_mutation_client_error_with_not_found(
                        e,
                        &redactor,
                        "admin attachment-settings update",
                        "the Wekan settings document was not found",
                    )
                })?;
            Ok(CommandSuccess::AttachmentSettingsUpdated(Box::new(
                AttachmentSettingsUpdateSuccess {
                    settings: document.into(),
                    submitted_fields,
                },
            )))
        }
    }
}

fn submitted_fields(request: &UpdateAttachmentSettings) -> Vec<AttachmentSettingField> {
    let mut fields = Vec::new();
    if request.default_storage.is_some() {
        fields.push(AttachmentSettingField::DefaultStorage);
    }
    if request.storage_config.is_some() {
        fields.push(AttachmentSettingField::StorageConfig);
    }
    if request.upload_settings.is_some() {
        fields.push(AttachmentSettingField::UploadSettings);
    }
    if request.limit_settings.is_some() {
        fields.push(AttachmentSettingField::LimitSettings);
    }
    if request.migration_settings.is_some() {
        fields.push(AttachmentSettingField::MigrationSettings);
    }
    fields
}
