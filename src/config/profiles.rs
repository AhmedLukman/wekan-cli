use std::{
    collections::BTreeMap,
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use directories::ProjectDirs;
use fs4::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::client::{ServerUrl, ServerUrlError};

const CONFIG_VERSION: u8 = 1;
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;
const APPLICATION_NAME: &str = "wekan-cli";
const CONFIG_FILE_NAME: &str = "profiles.json";
const LOCK_FILE_NAME: &str = "profiles.lock";
const BACKUP_FILE_NAME: &str = "profiles.json.bak";
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    server: String,
}

impl Profile {
    pub fn new(server: String) -> Self {
        Self { server }
    }

    pub fn server(&self) -> &str {
        &self.server
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProfileDocument {
    revision: u64,
    active_profile: Option<String>,
    profiles: BTreeMap<String, Profile>,
}

impl ProfileDocument {
    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    pub fn active_profile(&self) -> Option<&str> {
        self.active_profile.as_deref()
    }

    pub fn profiles(&self) -> &BTreeMap<String, Profile> {
        &self.profiles
    }

    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    pub fn insert(&mut self, name: String, profile: Profile) -> Option<Profile> {
        self.profiles.insert(name, profile)
    }

    pub fn remove(&mut self, name: &str) -> Option<Profile> {
        self.profiles.remove(name)
    }

    pub fn set_active_profile(&mut self, name: Option<String>) {
        self.active_profile = name;
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

pub trait ProfileLease: Send {}

impl ProfileLease for File {}

#[derive(Debug)]
struct NoopLease;

impl ProfileLease for NoopLease {}

pub struct ProfileSnapshot {
    document: ProfileDocument,
    _lease: Box<dyn ProfileLease>,
}

impl ProfileSnapshot {
    pub fn new_unlocked(document: ProfileDocument) -> Self {
        Self {
            document,
            _lease: Box::new(NoopLease),
        }
    }

    fn locked(document: ProfileDocument, lease: File) -> Self {
        Self {
            document,
            _lease: Box::new(lease),
        }
    }

    pub const fn document(&self) -> &ProfileDocument {
        &self.document
    }
}

pub trait ProfileMutation: Send {
    fn document(&self) -> &ProfileDocument;
    fn save(&mut self, document: ProfileDocument) -> Result<(), ProfileStoreError>;
}

pub trait ProfileStore: Send + Sync {
    fn read(&self) -> Result<ProfileSnapshot, ProfileStoreError>;
    fn lock_mutation(&self) -> Result<Box<dyn ProfileMutation + Send + '_>, ProfileStoreError>;
    fn credential_namespace(&self) -> Result<String, ProfileStoreError>;
}

#[derive(Clone, Debug)]
pub struct FileProfileStore {
    location: Result<PathBuf, String>,
}

impl FileProfileStore {
    pub fn production() -> Self {
        Self {
            location: profile_directory(env::var_os("WEKAN_CONFIG_DIR")),
        }
    }

    pub fn at(directory: PathBuf) -> Self {
        Self {
            location: Ok(directory),
        }
    }

    fn paths(&self) -> Result<ProfilePaths, ProfileStoreError> {
        let directory = self
            .location
            .as_ref()
            .map_err(|message| ProfileStoreError::Configuration(message.clone()))?
            .clone();
        Ok(ProfilePaths {
            config: directory.join(CONFIG_FILE_NAME),
            lock: directory.join(LOCK_FILE_NAME),
            backup: directory.join(BACKUP_FILE_NAME),
            directory,
        })
    }
}

impl Default for FileProfileStore {
    fn default() -> Self {
        Self::production()
    }
}

impl ProfileStore for FileProfileStore {
    fn read(&self) -> Result<ProfileSnapshot, ProfileStoreError> {
        let paths = self.paths()?;
        prepare_directory(&paths.directory)?;
        let lock = open_lock(&paths.lock)?;
        FileExt::lock_shared(&lock).map_err(ProfileStoreError::Lock)?;
        let document = load_document(&paths)?;
        Ok(ProfileSnapshot::locked(document, lock))
    }

    fn lock_mutation(&self) -> Result<Box<dyn ProfileMutation + Send + '_>, ProfileStoreError> {
        let paths = self.paths()?;
        prepare_directory(&paths.directory)?;
        let lock = open_lock(&paths.lock)?;
        FileExt::lock(&lock).map_err(ProfileStoreError::Lock)?;
        let document = load_document(&paths)?;
        Ok(Box::new(FileProfileMutation {
            paths,
            document,
            _lock: lock,
        }))
    }

    fn credential_namespace(&self) -> Result<String, ProfileStoreError> {
        let paths = self.paths()?;
        prepare_directory(&paths.directory)?;
        profile_store_namespace(&paths.directory)
    }
}

struct FileProfileMutation {
    paths: ProfilePaths,
    document: ProfileDocument,
    _lock: File,
}

impl ProfileMutation for FileProfileMutation {
    fn document(&self) -> &ProfileDocument {
        &self.document
    }

    fn save(&mut self, mut document: ProfileDocument) -> Result<(), ProfileStoreError> {
        if document.revision != self.document.revision {
            return Err(ProfileStoreError::Invalid(
                "the profile configuration changed before it could be saved".to_owned(),
            ));
        }
        document.revision = document.revision.checked_add(1).ok_or_else(|| {
            ProfileStoreError::Invalid("the profile configuration revision is exhausted".to_owned())
        })?;
        validate_document(&document)?;
        match persist_document(&self.paths, &document) {
            Ok(()) => {
                self.document = document;
                Ok(())
            }
            Err(error) if error.profile_was_installed() => {
                self.document = document;
                Err(error)
            }
            Err(error) => Err(error),
        }
    }
}

#[derive(Clone, Debug)]
struct ProfilePaths {
    directory: PathBuf,
    config: PathBuf,
    lock: PathBuf,
    backup: PathBuf,
}

#[derive(Debug, Error)]
pub enum ProfileStoreError {
    #[error("{0}")]
    Configuration(String),
    #[error("the profile configuration directory could not be prepared: {0}")]
    Directory(#[source] io::Error),
    #[error("the profile configuration lock could not be opened: {0}")]
    LockOpen(#[source] io::Error),
    #[error("the profile configuration lock could not be acquired: {0}")]
    Lock(#[source] io::Error),
    #[error("the profile configuration could not be inspected: {0}")]
    Inspect(#[source] io::Error),
    #[error("the profile configuration is larger than 1 MiB")]
    TooLarge,
    #[error("the profile configuration could not be read: {0}")]
    Read(#[source] io::Error),
    #[error("the profile configuration is not valid JSON: {0}")]
    Deserialize(#[source] serde_json::Error),
    #[error("profile configuration version {0} is not supported")]
    UnsupportedVersion(u8),
    #[error("the profile configuration is invalid: {0}")]
    Invalid(String),
    #[error("the profile configuration could not be serialized: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("the profile configuration could not be written safely: {0}")]
    Write(#[source] io::Error),
    #[error("the profile configuration was installed but could not be finalized safely: {0}")]
    WriteAfterInstall(#[source] io::Error),
}

impl ProfileStoreError {
    pub(crate) const fn profile_was_installed(&self) -> bool {
        matches!(self, Self::WriteAfterInstall(_))
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredDocument {
    version: u8,
    revision: u64,
    active_profile: Option<String>,
    profiles: BTreeMap<String, StoredProfile>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredProfile {
    server: String,
}

pub fn validate_profile_name(name: &str) -> Result<(), String> {
    let valid_length = (1..=64).contains(&name.len());
    let mut bytes = name.bytes();
    let valid_first = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
    let valid_rest = bytes.all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
    });

    if valid_length && valid_first && valid_rest {
        Ok(())
    } else {
        Err("profile names must be 1-64 characters and match [a-z0-9][a-z0-9._-]*".to_owned())
    }
}

pub fn parse_profile_name(raw: &str) -> Result<String, String> {
    validate_profile_name(raw)?;
    Ok(raw.to_owned())
}

pub fn canonical_server(raw: &str) -> Result<String, ServerUrlError> {
    ServerUrl::parse(raw).map(|url| url.as_str().to_owned())
}

fn profile_directory(override_directory: Option<OsString>) -> Result<PathBuf, String> {
    if let Some(raw) = override_directory {
        let directory = PathBuf::from(raw);
        if directory.as_os_str().is_empty() {
            return Err("WEKAN_CONFIG_DIR must not be empty".to_owned());
        }
        if !directory.is_absolute() {
            return Err("WEKAN_CONFIG_DIR must be an absolute path".to_owned());
        }
        return Ok(directory);
    }

    ProjectDirs::from("org", "Wekan", APPLICATION_NAME)
        .map(|directories| directories.config_dir().to_owned())
        .ok_or_else(|| "no platform configuration directory is available".to_owned())
}

fn profile_store_namespace(directory: &Path) -> Result<String, ProfileStoreError> {
    let canonical = fs::canonicalize(directory).map_err(ProfileStoreError::Inspect)?;
    let mut digest = Sha256::new();
    digest.update(b"wekan-cli-profile-store-v1\0");
    digest.update(profile_store_identity_bytes(&canonical));
    Ok(format!("sha256-{:x}", digest.finalize()))
}

#[cfg(unix)]
fn profile_store_identity_bytes(directory: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;

    directory.as_os_str().as_bytes().to_vec()
}

#[cfg(windows)]
fn profile_store_identity_bytes(directory: &Path) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;

    directory
        .as_os_str()
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[cfg(not(any(unix, windows)))]
fn profile_store_identity_bytes(directory: &Path) -> Vec<u8> {
    directory
        .as_os_str()
        .to_string_lossy()
        .into_owned()
        .into_bytes()
}

fn prepare_directory(directory: &Path) -> Result<(), ProfileStoreError> {
    if let Some(parent) = directory
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(ProfileStoreError::Directory)?;
    }

    let builder = fs::DirBuilder::new();
    #[cfg(unix)]
    let builder = {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = builder;
        builder.mode(0o700);
        builder
    };
    match builder.create(directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && directory.is_dir() => Ok(()),
        Err(error) => Err(ProfileStoreError::Directory(error)),
    }
}

fn open_lock(path: &Path) -> Result<File, ProfileStoreError> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(ProfileStoreError::LockOpen)?;
    restrict_file_permissions(path).map_err(ProfileStoreError::LockOpen)?;
    Ok(file)
}

fn metadata_if_present(path: &Path) -> io::Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::metadata(path).map(Some),
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn load_document(paths: &ProfilePaths) -> Result<ProfileDocument, ProfileStoreError> {
    let (path, metadata) = if let Some(metadata) =
        metadata_if_present(&paths.config).map_err(ProfileStoreError::Inspect)?
    {
        (&paths.config, metadata)
    } else if let Some(metadata) =
        metadata_if_present(&paths.backup).map_err(ProfileStoreError::Inspect)?
    {
        (&paths.backup, metadata)
    } else {
        return Ok(ProfileDocument::default());
    };

    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(ProfileStoreError::TooLarge);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .map_err(ProfileStoreError::Read)?
        .take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(ProfileStoreError::Read)?;
    if bytes.len() as u64 > MAX_CONFIG_BYTES {
        return Err(ProfileStoreError::TooLarge);
    }

    let stored: StoredDocument =
        serde_json::from_slice(&bytes).map_err(ProfileStoreError::Deserialize)?;
    if stored.version != CONFIG_VERSION {
        return Err(ProfileStoreError::UnsupportedVersion(stored.version));
    }
    let document = ProfileDocument {
        revision: stored.revision,
        active_profile: stored.active_profile,
        profiles: stored
            .profiles
            .into_iter()
            .map(|(name, profile)| (name, Profile::new(profile.server)))
            .collect(),
    };
    validate_document(&document)?;
    Ok(document)
}

fn validate_document(document: &ProfileDocument) -> Result<(), ProfileStoreError> {
    for (name, profile) in document.profiles() {
        validate_profile_name(name).map_err(ProfileStoreError::Invalid)?;
        let canonical = canonical_server(profile.server()).map_err(|error| {
            ProfileStoreError::Invalid(format!("profile `{name}` has an invalid server: {error}"))
        })?;
        if canonical != profile.server() {
            return Err(ProfileStoreError::Invalid(format!(
                "profile `{name}` does not contain a canonical server URL"
            )));
        }
    }
    if let Some(active) = document.active_profile()
        && !document.profiles().contains_key(active)
    {
        return Err(ProfileStoreError::Invalid(format!(
            "active profile `{active}` does not exist"
        )));
    }
    Ok(())
}

fn persist_document(
    paths: &ProfilePaths,
    document: &ProfileDocument,
) -> Result<(), ProfileStoreError> {
    let stored = StoredDocument {
        version: CONFIG_VERSION,
        revision: document.revision,
        active_profile: document.active_profile.clone(),
        profiles: document
            .profiles
            .iter()
            .map(|(name, profile)| {
                (
                    name.clone(),
                    StoredProfile {
                        server: profile.server.clone(),
                    },
                )
            })
            .collect(),
    };
    let mut bytes = serde_json::to_vec_pretty(&stored).map_err(ProfileStoreError::Serialize)?;
    bytes.push(b'\n');
    if bytes.len() as u64 > MAX_CONFIG_BYTES {
        return Err(ProfileStoreError::TooLarge);
    }

    let (temp_path, mut temp) = loop {
        let path = paths.directory.join(format!(
            ".profiles.json.{}.{}.tmp",
            std::process::id(),
            TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => break (path, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(ProfileStoreError::Write(error)),
        }
    };
    if let Err(error) = restrict_file_permissions(&temp_path)
        .and_then(|()| temp.write_all(&bytes))
        .and_then(|()| temp.sync_all())
    {
        let _ = fs::remove_file(&temp_path);
        return Err(ProfileStoreError::Write(error));
    }
    drop(temp);

    let result = replace_with_backup(paths, &temp_path);
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn replace_with_backup(paths: &ProfilePaths, temp_path: &Path) -> Result<(), ProfileStoreError> {
    let before_install = ProfileStoreError::Write;
    let after_install = ProfileStoreError::WriteAfterInstall;
    let had_config = metadata_if_present(&paths.config)
        .map_err(before_install)?
        .is_some();
    if had_config {
        if metadata_if_present(&paths.backup)
            .map_err(before_install)?
            .is_some()
        {
            fs::remove_file(&paths.backup).map_err(before_install)?;
        }
        fs::rename(&paths.config, &paths.backup).map_err(before_install)?;
        sync_directory(&paths.directory).map_err(before_install)?;
    }

    if let Err(error) = fs::rename(temp_path, &paths.config) {
        if had_config {
            let _ = fs::rename(&paths.backup, &paths.config);
        }
        return Err(before_install(error));
    }

    sync_directory(&paths.directory).map_err(after_install)?;
    if metadata_if_present(&paths.backup)
        .map_err(after_install)?
        .is_some()
    {
        // The new configuration is already committed. Failure to remove a
        // stale backup is safe; it will be cleaned after a later mutation.
        let _ = fs::remove_file(&paths.backup);
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> io::Result<()> {
    File::open(directory)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_directory: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn restrict_file_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_file_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests;
