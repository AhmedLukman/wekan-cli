# Global administrator settings

For domain reporting and organization/team feature controls, see
[administrator commands](admin.md).

These commands use Wekan v11.06's native REST endpoints and require site
administration. `admin settings get/update` calls `/api/settings`;
`admin attachment-settings get/update` calls `/api/admin/attachment-settings`.
They configure stored settings; success does not verify cloud connectivity,
move attachments, or confirm migration completion.

```console
wekan admin settings get
wekan admin settings update --product-name "Team boards" --disable-registration false
wekan admin attachment-settings get
wekan admin attachment-settings update --settings-file attachment-settings.json
wekan admin attachment-settings update --s3-enabled false --s3-bucket backups --s3-secret-access-key-file s3-key.txt
```

## Update input

Supply at least one named setting flag, `--settings-file <PATH>`, or
`--settings-stdin`. JSON source options are mutually exclusive and conflict
with every setting flag. Files must be regular UTF-8 files. Both acquisition
and serialized requests are limited to 50 MiB. Boolean flags require `true`
or `false`; numeric flags accept finite JSON numbers, including fractions,
within the setting's range. Other than MIME types, setting flags cannot repeat.

JSON uses the camelCase keys in the tables below. It rejects unknown fields,
read-only IDs/timestamps/actor metadata, credential `Set` markers, wrong types,
empty top-level updates, and invalid null inputs before sending a request.
Global JSON updates accept explicit null for all 34 optional whitelisted fields;
`defaultAuthenticationMethod` must remain a string. Omission leaves a field unchanged,
while null is sent and persisted. Attachment settings retain their existing null
rules. Explicit nested objects, including empty objects, retain native semantics.

All four commands accept `--server <URL>`, `--profile <PROFILE>`,
`--output human|json`, `--allow-insecure-http`, and `--help`.

## Global settings

The table is the complete REST allowlist. Other Admin Panel controls, including
SMTP/mailServer, theme controls, and general feature toggles absent from this
list, cannot be changed through these commands. Omitted fields remain unchanged;
strings are sent to Wekan unchanged for its schema cleaning and validation.

| JSON key | Flag | Value |
| --- | --- | --- |
| `disableRegistration` | `--disable-registration` | boolean |
| `disableForgotPassword` | `--disable-forgot-password` | boolean |
| `productName` | `--product-name` | string |
| `displayAuthenticationMethod` | `--display-authentication-method` | boolean |
| `defaultAuthenticationMethod` | `--default-authentication-method` | string |
| `spinnerName` | `--spinner-name` | string |
| `hideLogo` | `--hide-logo` | boolean |
| `hideCardCounterList` | `--hide-card-counter-list` | boolean |
| `hideBoardMemberList` | `--hide-board-member-list` | boolean |
| `customLoginLogoImageUrl` | `--custom-login-logo-image-url` | string |
| `customLoginLogoLinkUrl` | `--custom-login-logo-link-url` | string |
| `customHelpLinkUrl` | `--custom-help-link-url` | string |
| `textBelowCustomLoginLogo` | `--text-below-custom-login-logo` | string |
| `automaticLinkedUrlSchemes` | `--automatic-linked-url-schemes` | string |
| `customTopLeftCornerLogoImageUrl` | `--custom-top-left-corner-logo-image-url` | string |
| `customTopLeftCornerLogoLinkUrl` | `--custom-top-left-corner-logo-link-url` | string |
| `customTopLeftCornerLogoHeight` | `--custom-top-left-corner-logo-height` | string |
| `oidcBtnText` | `--oidc-btn-text` | string |
| `mailDomainName` | `--mail-domain-name` | string |
| `legalNotice` | `--legal-notice` | string |
| `customHeadEnabled` | `--custom-head-enabled` | boolean |
| `customHeadMetaTags` | `--custom-head-meta-tags` | string |
| `customHeadLinkTags` | `--custom-head-link-tags` | string |
| `customManifestEnabled` | `--custom-manifest-enabled` | boolean |
| `customManifestContent` | `--custom-manifest-content` | string |
| `customAssetLinksEnabled` | `--custom-asset-links-enabled` | boolean |
| `customAssetLinksContent` | `--custom-asset-links-content` | string |
| `accessibilityPageEnabled` | `--accessibility-page-enabled` | boolean |
| `accessibilityTitle` | `--accessibility-title` | string |
| `accessibilityContent` | `--accessibility-content` | string |
| `supportPopupText` | `--support-popup-text` | string |
| `supportPageEnabled` | `--support-page-enabled` | boolean |
| `supportPagePublic` | `--support-page-public` | boolean |
| `supportTitle` | `--support-title` | string |
| `supportPageText` | `--support-page-text` | string |

## Attachment-storage settings

`defaultStorage` accepts `fs`, `gridfs`, `s3`, `azure`, or `gcs`. Cloud providers
support their known boolean/string options below. The upstream provider objects
are open-ended; this CLI deliberately rejects unmodeled properties and invalid
known-field types in responses with `protocol_error`. The existing raw
`api request` command preserves arbitrary provider responses when needed.

| JSON path | Flag | Value |
| --- | --- | --- |
| `defaultStorage` | `--default-storage` | backend |
| `storageConfig.filesystem.enabled` | `--filesystem-enabled` | boolean |
| `storageConfig.filesystem.read` | `--filesystem-read` | boolean |
| `storageConfig.filesystem.write` | `--filesystem-write` | boolean |
| `storageConfig.filesystem.path` | `--filesystem-path` | string |
| `storageConfig.gridfs.enabled` | `--gridfs-enabled` | boolean |
| `storageConfig.gridfs.read` | `--gridfs-read` | boolean |
| `storageConfig.gridfs.write` | `--gridfs-write` | boolean |
| `storageConfig.s3.enabled` | `--s3-enabled` | boolean |
| `storageConfig.s3.read` | `--s3-read` | boolean |
| `storageConfig.s3.write` | `--s3-write` | boolean |
| `storageConfig.s3.endpoint` | `--s3-endpoint` | string |
| `storageConfig.s3.region` | `--s3-region` | string |
| `storageConfig.s3.bucket` | `--s3-bucket` | string |
| `storageConfig.s3.accessKeyId` | `--s3-access-key-id` | string |
| `storageConfig.s3.forcePathStyle` | `--s3-force-path-style` | boolean |
| `storageConfig.azure.enabled` | `--azure-enabled` | boolean |
| `storageConfig.azure.read` | `--azure-read` | boolean |
| `storageConfig.azure.write` | `--azure-write` | boolean |
| `storageConfig.azure.accountName` | `--azure-account-name` | string |
| `storageConfig.azure.bucket` | `--azure-bucket` | string |
| `storageConfig.gcs.enabled` | `--gcs-enabled` | boolean |
| `storageConfig.gcs.read` | `--gcs-read` | boolean |
| `storageConfig.gcs.write` | `--gcs-write` | boolean |
| `storageConfig.gcs.projectId` | `--gcs-project-id` | string |
| `storageConfig.gcs.keyFilename` | `--gcs-key-filename` | string |
| `storageConfig.gcs.bucket` | `--gcs-bucket` | string |
| `uploadSettings.maxFileSize` | `--upload-max-file-size` | number |
| `uploadSettings.allowedMimeTypes` | `--upload-allowed-mime-type` | repeatable string |
| `limitSettings.attachmentsUploadMaxBytes` | `--attachments-upload-max-bytes` | number |
| `limitSettings.attachmentsUploadBlocked` | `--attachments-upload-blocked` | boolean |
| `limitSettings.avatarsUploadBlocked` | `--avatars-upload-blocked` | boolean |
| `limitSettings.attachmentsDownloadMaxBytes` | `--attachments-download-max-bytes` | number |
| `limitSettings.attachmentsDownloadBlocked` | `--attachments-download-blocked` | boolean |
| `limitSettings.apiUploadMaxBytes` | `--api-upload-max-bytes` | number |
| `limitSettings.apiUploadBlocked` | `--api-upload-blocked` | boolean |
| `limitSettings.apiDownloadMaxBytes` | `--api-download-max-bytes` | number |
| `limitSettings.apiDownloadBlocked` | `--api-download-blocked` | boolean |
| `migrationSettings.autoMigrate` | `--migration-auto-migrate` | boolean |
| `migrationSettings.batchSize` | `--migration-batch-size` | number |
| `migrationSettings.delayMs` | `--migration-delay-ms` | number |
| `migrationSettings.cpuThreshold` | `--migration-cpu-threshold` | number |
| `storageConfig.s3.secretAccessKey` | `--s3-secret-access-key-file` | credential file |
| `storageConfig.s3.secretAccessKey` | `--s3-secret-access-key-stdin` | switch |
| `storageConfig.azure.accountKey` | `--azure-account-key-file` | credential file |
| `storageConfig.azure.accountKey` | `--azure-account-key-stdin` | switch |
| `storageConfig.azure.connectionString` | `--azure-connection-string-file` | credential file |
| `storageConfig.azure.connectionString` | `--azure-connection-string-stdin` | switch |
| `storageConfig.gcs.credentials` | `--gcs-credentials-file` | credential file |
| `storageConfig.gcs.credentials` | `--gcs-credentials-stdin` | switch |
| `uploadSettings.allowedMimeTypes` | `--clear-upload-allowed-mime-types` | switch |

`--upload-allowed-mime-type` replaces the complete MIME list; repeat it for each
entry. It conflicts with `--clear-upload-allowed-mime-types`, which sends `[]`.
Transfer byte limits have minimum zero; zero means unlimited. Migration batch
size ranges from 1–100, delay from 100–10000 ms, and CPU threshold from 10–90.
`uploadSettings.maxFileSize` retains the upstream number schema without an
additional CLI minimum.

### Native update semantics

Each command sends one request. Omitted top-level sections remain unchanged.
Supplying `limitSettings` or `uploadSettings` replaces that section: omitted
non-default siblings can disappear, and defaulted upload-block flags become
false. Supplying filesystem, GridFS, or migration objects can reset omitted
children to schema defaults. Cloud provider objects merge with their existing
configuration, preserving omitted providers and omitted provider properties.
No client read/merge, retry, or workaround changes these behaviors.

For example, sending `{"limitSettings":{"avatarsUploadBlocked":true}}` can
remove existing byte limits and reset other block flags. Include every desired
value for a replacement section in the same update.

### Credentials

The four secret fields are S3 `secretAccessKey`, Azure `accountKey` and
`connectionString`, and GCS `credentials`. Each has conflicting file/stdin flag
alternatives. Multiple files are allowed, but only one credential stdin option
can be selected. File and stdin credential inputs remove one final LF/CRLF;
other bytes and whitespace are preserved. Blank values preserve stored secrets;
these endpoints do not provide secret clearing. There are no literal argv
secret flags.

Full JSON file/stdin mode can supply multiple credentials. S3/Azure secrets
are strings. GCS credentials accept a string or opaque JSON object; dedicated
GCS file/stdin flags send their contents as a string. A GCS key filename is a
path on the Wekan server, not a local credential upload.

Responses must contain empty masked secret placeholders and boolean `Set`
markers. CLI output exposes only the markers, such as `secret_access_key_set`,
`account_key_set`, `connection_string_set`, and `credentials_set`. Credential
contents are excluded from debug output and redacted from returned errors.

## Results and errors

Reads and updates return `settings` with snake_case fields. `_id` becomes
`settings_id`; attachment timestamps and actor metadata remain available.
Optional fields serialize as null when absent; `upload_settings.allowed_mime_types`
is an array or null. Updates add `submitted_fields`
in canonical order, listing the top-level fields explicitly supplied; server
filled defaults do not add submitted fields. Human output includes the complete
mapped configuration with terminal controls escaped. Results describe stored
configuration, not a connectivity check or completed storage migration.

The global endpoint can return an empty document, represented as settings with
null fields; updating a missing global document returns HTTP 404. Attachment
GET can initialize a missing document through the native server path.
Authentication and application failures may arrive with HTTP 200. Errors retain
the real HTTP status, embedded Wekan status and error metadata. Unmapped fields,
invalid types, invalid dates, or unmasked secrets cause protocol errors.
Failed updates report server and protocol diagnostics.
