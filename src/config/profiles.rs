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
        persist_document(&self.paths, &document)?;
        self.document = document;
        Ok(())
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
    result.map_err(ProfileStoreError::Write)
}

fn replace_with_backup(paths: &ProfilePaths, temp_path: &Path) -> io::Result<()> {
    let had_config = metadata_if_present(&paths.config)?.is_some();
    if had_config {
        if metadata_if_present(&paths.backup)?.is_some() {
            fs::remove_file(&paths.backup)?;
        }
        fs::rename(&paths.config, &paths.backup)?;
        sync_directory(&paths.directory)?;
    }

    if let Err(error) = fs::rename(temp_path, &paths.config) {
        if had_config {
            let _ = fs::rename(&paths.backup, &paths.config);
        }
        return Err(error);
    }

    sync_directory(&paths.directory)?;
    if metadata_if_present(&paths.backup)?.is_some() {
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
mod tests {
    use std::{
        fs,
        io::{BufRead, BufReader, Write},
        process::{Command, Stdio},
        sync::mpsc,
        thread,
        time::Duration,
    };

    use super::{
        BACKUP_FILE_NAME, CONFIG_FILE_NAME, FileProfileStore, MAX_CONFIG_BYTES, Profile,
        ProfileDocument, ProfileStore, ProfileStoreError, canonical_server, load_document,
        profile_directory, replace_with_backup, validate_profile_name,
    };

    const LOCK_HELPER_DIRECTORY_ENV: &str = "WEKAN_TEST_PROFILE_LOCK_DIRECTORY";
    const LOCK_HELPER_READY: &str = "WEKAN_PROFILE_LOCK_READY";

    struct TestDirectory(std::path::PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "wekan-cli-profile-{label}-{}-{}",
                std::process::id(),
                super::TEMP_FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn document() -> ProfileDocument {
        let mut document = ProfileDocument::default();
        document.insert(
            "local".to_owned(),
            Profile::new("http://localhost:3000/".to_owned()),
        );
        document.insert(
            "work".to_owned(),
            Profile::new("https://wekan.example/".to_owned()),
        );
        document.set_active_profile(Some("local".to_owned()));
        document
    }

    #[test]
    fn validates_portable_profile_names() {
        for valid in ["a", "local-dev", "prod.eu_1", &"a".repeat(64)] {
            assert!(validate_profile_name(valid).is_ok(), "{valid}");
        }
        for invalid in ["", "Upper", "-local", "has space", "café", &"a".repeat(65)] {
            assert!(validate_profile_name(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn canonicalizes_server_identity_without_network_permission() {
        assert_eq!(
            canonical_server("https://EXAMPLE.com/wekan").unwrap(),
            "https://example.com/wekan/"
        );
        assert_eq!(
            canonical_server("http://wekan.example").unwrap(),
            "http://wekan.example/"
        );
    }

    #[test]
    fn absent_store_is_empty_and_round_trips_sorted_profiles() {
        let directory = TestDirectory::new("round-trip");
        let store = FileProfileStore::at(directory.path().to_owned());
        assert!(store.read().unwrap().document().is_empty());

        store.lock_mutation().unwrap().save(document()).unwrap();
        let snapshot = store.read().unwrap();
        let names = snapshot
            .document()
            .profiles()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert_eq!(names, ["local", "work"]);
        assert_eq!(snapshot.document().active_profile(), Some("local"));

        let encoded = fs::read_to_string(directory.path().join(CONFIG_FILE_NAME)).unwrap();
        assert!(encoded.contains("\"version\": 1"));
        assert!(encoded.contains("\"revision\": 1"));
        assert!(!encoded.contains("token"));
    }

    #[test]
    fn rejects_unknown_fields_unsupported_versions_and_oversized_files() {
        let directory = TestDirectory::new("invalid");
        let path = directory.path().join(CONFIG_FILE_NAME);
        fs::write(
            &path,
            r#"{"version":1,"revision":0,"active_profile":null,"profiles":{},"unknown":true}"#,
        )
        .unwrap();
        let store = FileProfileStore::at(directory.path().to_owned());
        assert!(matches!(
            store.read(),
            Err(ProfileStoreError::Deserialize(_))
        ));

        fs::write(
            &path,
            r#"{"version":0,"revision":0,"active_profile":null,"profiles":{}}"#,
        )
        .unwrap();
        assert!(matches!(
            store.read(),
            Err(ProfileStoreError::UnsupportedVersion(0))
        ));

        fs::write(&path, vec![b' '; MAX_CONFIG_BYTES as usize + 1]).unwrap();
        assert!(matches!(store.read(), Err(ProfileStoreError::TooLarge)));
    }

    #[test]
    fn rejects_noncanonical_records_and_missing_active_profiles() {
        let directory = TestDirectory::new("validation");
        let path = directory.path().join(CONFIG_FILE_NAME);
        fs::write(
            &path,
            r#"{"version":1,"revision":0,"active_profile":"missing","profiles":{"local":{"server":"https://example.com"}}}"#,
        )
        .unwrap();
        let store = FileProfileStore::at(directory.path().to_owned());
        assert!(matches!(store.read(), Err(ProfileStoreError::Invalid(_))));
    }

    #[test]
    fn reads_the_backup_after_an_interrupted_replace() {
        let directory = TestDirectory::new("backup");
        fs::write(
            directory.path().join(BACKUP_FILE_NAME),
            r#"{"version":1,"revision":0,"active_profile":"local","profiles":{"local":{"server":"http://localhost:3000/"}}}"#,
        )
        .unwrap();
        let store = FileProfileStore::at(directory.path().to_owned());
        assert_eq!(
            store.read().unwrap().document().active_profile(),
            Some("local")
        );

        let mut mutation = store.lock_mutation().unwrap();
        mutation.save(mutation.document().clone()).unwrap();
        assert!(directory.path().join(CONFIG_FILE_NAME).exists());
        let encoded = fs::read_to_string(directory.path().join(CONFIG_FILE_NAME)).unwrap();
        assert!(encoded.contains("\"version\": 1"));
        assert!(encoded.contains("\"revision\": 1"));
    }

    #[test]
    fn primary_inspection_errors_do_not_fall_back_to_backup() {
        let directory = TestDirectory::new("primary-inspection-error");

        let store = FileProfileStore::at(directory.path().to_owned());
        let mut paths = store.paths().unwrap();
        paths.config = directory.path().join("invalid\0primary");
        fs::write(
            &paths.backup,
            r#"{"version":1,"revision":0,"active_profile":null,"profiles":{}}"#,
        )
        .unwrap();

        assert!(matches!(
            load_document(&paths),
            Err(ProfileStoreError::Inspect(_))
        ));
    }

    #[test]
    fn backup_inspection_errors_do_not_make_store_empty() {
        let directory = TestDirectory::new("backup-inspection-error");

        let store = FileProfileStore::at(directory.path().to_owned());
        let mut paths = store.paths().unwrap();
        paths.backup = directory.path().join("invalid\0backup");

        assert!(matches!(
            load_document(&paths),
            Err(ProfileStoreError::Inspect(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn dangling_primary_link_does_not_fall_back_to_backup() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new("dangling-primary-link");
        let store = FileProfileStore::at(directory.path().to_owned());
        let paths = store.paths().unwrap();
        symlink("missing-profile-document", &paths.config).unwrap();
        fs::write(
            &paths.backup,
            r#"{"version":1,"revision":0,"active_profile":null,"profiles":{}}"#,
        )
        .unwrap();

        assert!(matches!(
            load_document(&paths),
            Err(ProfileStoreError::Inspect(_))
        ));
    }

    #[test]
    fn failed_recovery_replace_preserves_the_only_backup() {
        let directory = TestDirectory::new("backup-failure");
        fs::write(
            directory.path().join(BACKUP_FILE_NAME),
            r#"{"version":1,"revision":0,"active_profile":"local","profiles":{"local":{"server":"http://localhost:3000/"}}}"#,
        )
        .unwrap();
        let store = FileProfileStore::at(directory.path().to_owned());
        let paths = store.paths().unwrap();

        assert!(replace_with_backup(&paths, &directory.path().join("missing.tmp")).is_err());
        assert!(paths.backup.exists());
        assert_eq!(
            store.read().unwrap().document().active_profile(),
            Some("local")
        );
    }

    #[cfg(unix)]
    #[test]
    fn replacement_preserves_backup_when_primary_cannot_be_inspected() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new("replace-inspection-error");
        let store = FileProfileStore::at(directory.path().to_owned());
        let paths = store.paths().unwrap();
        symlink(CONFIG_FILE_NAME, &paths.config).unwrap();
        fs::write(&paths.backup, "previous backup").unwrap();
        let temp_path = directory.path().join("replacement.tmp");
        fs::write(&temp_path, "replacement").unwrap();

        assert!(replace_with_backup(&paths, &temp_path).is_err());
        assert!(
            fs::symlink_metadata(&paths.config)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(&paths.backup).unwrap(),
            "previous backup"
        );
        assert_eq!(fs::read_to_string(&temp_path).unwrap(), "replacement");
    }

    #[test]
    fn shared_snapshot_blocks_an_exclusive_mutation() {
        let directory = TestDirectory::new("thread-lock");
        let store = FileProfileStore::at(directory.path().to_owned());
        let snapshot = store.read().unwrap();
        let other_store = store.clone();
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let _mutation = other_store.lock_mutation().unwrap();
            sender.send(()).unwrap();
        });
        assert!(receiver.recv_timeout(Duration::from_millis(150)).is_err());
        drop(snapshot);
        receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn config_directory_override_must_be_absolute() {
        assert!(profile_directory(Some("relative".into())).is_err());
        let directory = TestDirectory::new("override");
        assert_eq!(
            profile_directory(Some(directory.path().as_os_str().to_owned())).unwrap(),
            directory.path()
        );
    }

    #[test]
    fn persistent_profile_stores_have_stable_distinct_credential_namespaces() {
        let first_directory = TestDirectory::new("credential-namespace-first");
        let second_directory = TestDirectory::new("credential-namespace-second");
        let first = FileProfileStore::at(first_directory.path().to_owned());
        let second = FileProfileStore::at(second_directory.path().to_owned());

        let first_namespace = first.credential_namespace().unwrap();
        assert_eq!(first.credential_namespace().unwrap(), first_namespace);
        assert_ne!(first_namespace, second.credential_namespace().unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn store_uses_private_unix_permissions_for_created_paths() {
        use std::os::unix::fs::PermissionsExt;

        let root = TestDirectory::new("permissions");
        let directory = root.path().join("created");
        let store = FileProfileStore::at(directory.clone());
        store.lock_mutation().unwrap().save(document()).unwrap();
        assert_eq!(
            fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(directory.join(CONFIG_FILE_NAME))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[cfg(unix)]
    #[test]
    fn store_preserves_preexisting_unix_directory_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new("existing-permissions");
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o750)).unwrap();
        let store = FileProfileStore::at(directory.path().to_owned());
        drop(store.read().unwrap());

        assert_eq!(
            fs::metadata(directory.path()).unwrap().permissions().mode() & 0o777,
            0o750
        );
    }

    #[test]
    #[ignore = "subprocess helper invoked by the cross-process profile lock test"]
    fn profile_lock_process_helper() {
        let Some(directory) = std::env::var_os(LOCK_HELPER_DIRECTORY_ENV) else {
            return;
        };
        let store = FileProfileStore::at(directory.into());
        let _mutation = store.lock_mutation().unwrap();
        println!("{LOCK_HELPER_READY}");
        std::io::stdout().flush().unwrap();
    }

    #[test]
    fn profile_lock_excludes_another_process() {
        let directory = TestDirectory::new("process-lock");
        let store = FileProfileStore::at(directory.path().to_owned());
        let snapshot = store.read().unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "config::profiles::tests::profile_lock_process_helper",
                "--ignored",
                "--nocapture",
            ])
            .env(LOCK_HELPER_DIRECTORY_ENV, directory.path())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut sender = Some(sender);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 {
                    return;
                }
                if line.contains(LOCK_HELPER_READY) {
                    if let Some(sender) = sender.take() {
                        sender.send(line).unwrap();
                    }
                }
            }
        });
        assert!(receiver.recv_timeout(Duration::from_millis(200)).is_err());
        drop(snapshot);
        assert!(
            receiver
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .contains(LOCK_HELPER_READY)
        );
        assert!(child.wait().unwrap().success());
    }
}
