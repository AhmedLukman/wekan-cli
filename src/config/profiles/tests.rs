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
