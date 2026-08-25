use std::{
    fs::{self, File, OpenOptions},
    io,
    path::PathBuf,
};

use directories::ProjectDirs;
use fs4::FileExt;
use keyring::Entry;

use super::{
    CredentialDeleteOutcome, CredentialError, CredentialMutation, CredentialRecord, CredentialStore,
};

const SERVICE_NAME: &str = "wekan-cli";
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Copy, Debug, Default)]
pub struct KeyringCredentialStore;

impl CredentialStore for KeyringCredentialStore {
    fn check_available(&self, account: &str) -> Result<(), CredentialError> {
        Entry::store_status()
            .as_ref()
            .map_err(|error| CredentialError::Unavailable(error.to_string()))?;
        Entry::new(SERVICE_NAME, account)
            .map(|_| ())
            .map_err(|error| CredentialError::Unavailable(error.to_string()))
    }

    fn load(&self, account: &str) -> Result<Option<CredentialRecord>, CredentialError> {
        load_record(account)
    }

    fn save(&self, account: &str, record: &CredentialRecord) -> Result<(), CredentialError> {
        self.lock_mutation(account)?.save(record)
    }

    fn delete(&self, account: &str) -> Result<bool, CredentialError> {
        self.lock_mutation(account)?.delete()
    }

    fn delete_if_matches(
        &self,
        account: &str,
        expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        self.lock_mutation(account)?.delete_if_matches(expected)
    }

    fn lock_mutation(
        &self,
        account: &str,
    ) -> Result<Box<dyn CredentialMutation + Send + '_>, CredentialError> {
        Ok(Box::new(KeyringCredentialMutation {
            account: account.to_owned(),
            _lock: acquire_mutation_lock(account)?,
        }))
    }
}

struct KeyringCredentialMutation {
    account: String,
    // Keep the native file handle alive for the full credential transaction.
    _lock: File,
}

impl CredentialMutation for KeyringCredentialMutation {
    fn load(&self) -> Result<Option<CredentialRecord>, CredentialError> {
        load_record(&self.account)
    }

    fn save(&self, record: &CredentialRecord) -> Result<(), CredentialError> {
        save_record(&self.account, &record.encode()?)
    }

    fn delete(&self) -> Result<bool, CredentialError> {
        delete_record(&self.account)
    }

    fn delete_if_matches(
        &self,
        expected: &CredentialRecord,
    ) -> Result<CredentialDeleteOutcome, CredentialError> {
        let Some(current) = self.load()? else {
            return Ok(CredentialDeleteOutcome::Absent);
        };
        if !current.matches(expected) {
            return Ok(CredentialDeleteOutcome::Mismatch);
        }

        self.delete().map(|removed| {
            if removed {
                CredentialDeleteOutcome::Removed
            } else {
                CredentialDeleteOutcome::Absent
            }
        })
    }
}

fn load_record(account: &str) -> Result<Option<CredentialRecord>, CredentialError> {
    let entry = Entry::new(SERVICE_NAME, account).map_err(map_load_error)?;
    match entry.get_secret() {
        Ok(encoded) => CredentialRecord::decode(account, &encoded).map(Some),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(map_load_error(error)),
    }
}

fn save_record(account: &str, encoded: &[u8]) -> Result<(), CredentialError> {
    let entry = Entry::new(SERVICE_NAME, account)
        .map_err(|error| CredentialError::Store(error.to_string()))?;
    entry
        .set_secret(encoded)
        .map_err(|error| CredentialError::Store(error.to_string()))
}

fn delete_record(account: &str) -> Result<bool, CredentialError> {
    let entry = Entry::new(SERVICE_NAME, account)
        .map_err(|error| CredentialError::Delete(error.to_string()))?;
    match entry.delete_credential() {
        Ok(()) => Ok(true),
        Err(error) => map_delete_error(error),
    }
}

fn acquire_mutation_lock(account: &str) -> Result<File, CredentialError> {
    let lock = open_mutation_lock(account)
        .map_err(|error| CredentialError::Lock(format!("could not open the lock: {error}")))?;
    FileExt::lock(&lock)
        .map_err(|error| CredentialError::Lock(format!("could not acquire the lock: {error}")))?;
    Ok(lock)
}

fn open_mutation_lock(account: &str) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(mutation_lock_path(account)?)
}

fn mutation_lock_path(account: &str) -> io::Result<PathBuf> {
    // The stable hash keeps URL punctuation out of the filename. A collision
    // only serializes unrelated accounts and cannot expose credential data.
    let mut hash = FNV_OFFSET_BASIS;
    for byte in account.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }

    Ok(credential_lock_directory()?.join(format!("wekan-cli-credential-{hash:016x}.lock")))
}

fn credential_lock_directory() -> io::Result<PathBuf> {
    let directories = ProjectDirs::from("org", "Wekan", SERVICE_NAME).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no local application-data directory is available",
        )
    })?;
    let directory = directories.data_local_dir().join("credential-locks");
    fs::create_dir_all(&directory)?;
    restrict_directory_permissions(&directory)?;
    Ok(directory)
}

#[cfg(unix)]
fn restrict_directory_permissions(directory: &std::path::Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn restrict_directory_permissions(_directory: &std::path::Path) -> io::Result<()> {
    Ok(())
}

fn map_delete_error(error: keyring::Error) -> Result<bool, CredentialError> {
    match error {
        keyring::Error::NoEntry => Ok(false),
        error => Err(CredentialError::Delete(error.to_string())),
    }
}

fn map_load_error(error: keyring::Error) -> CredentialError {
    let message = error.to_string();
    match error {
        keyring::Error::NoStorageAccess(_)
        | keyring::Error::PlatformFailure(_)
        | keyring::Error::NoDefaultStore
        | keyring::Error::NotSupportedByStore(_) => CredentialError::Unavailable(message),
        _ => CredentialError::Load(message),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Write},
        process::{Command, Stdio},
        sync::mpsc,
        thread,
        time::Duration,
    };

    use directories::ProjectDirs;
    use fs4::{FileExt, TryLockError};

    use super::{
        acquire_mutation_lock, map_delete_error, map_load_error, mutation_lock_path,
        open_mutation_lock,
    };
    use crate::credentials::CredentialError;

    const LOCK_PROBE_ACCOUNT_ENV: &str = "WEKAN_TEST_CREDENTIAL_LOCK_PROBE_ACCOUNT";
    const LOCK_PROBE_PREFIX: &str = "WEKAN_LOCK_PROBE:";

    #[test]
    #[ignore = "subprocess helper invoked by the cross-process lock test"]
    fn credential_lock_process_helper() {
        let account = std::env::var(LOCK_PROBE_ACCOUNT_ENV).unwrap();
        let lock = open_mutation_lock(&account).unwrap();

        assert!(matches!(
            FileExt::try_lock(&lock),
            Err(TryLockError::WouldBlock)
        ));
        println!("{LOCK_PROBE_PREFIX}contended");
        std::io::stdout().flush().unwrap();

        FileExt::lock(&lock).unwrap();
        println!("{LOCK_PROBE_PREFIX}acquired");
        std::io::stdout().flush().unwrap();
    }

    #[test]
    fn credential_mutation_lock_excludes_another_process_until_release() {
        let account = format!("https://process-lock-test-{}.invalid/", std::process::id());
        let path = mutation_lock_path(&account).unwrap();
        let parent_lock = acquire_mutation_lock(&account).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "credentials::keyring::tests::credential_lock_process_helper",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(LOCK_PROBE_ACCOUNT_ENV, &account)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (state_sender, state_receiver) = mpsc::channel();
        let output_reader = thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let line = line.unwrap();
                if let Some((_, state)) = line.split_once(LOCK_PROBE_PREFIX)
                    && state_sender.send(state.to_owned()).is_err()
                {
                    break;
                }
            }
        });

        assert_eq!(
            state_receiver
                .recv_timeout(Duration::from_secs(10))
                .unwrap(),
            "contended"
        );
        drop(parent_lock);
        assert_eq!(
            state_receiver
                .recv_timeout(Duration::from_secs(10))
                .unwrap(),
            "acquired"
        );

        assert!(child.wait().unwrap().success());
        output_reader.join().unwrap();
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn credential_mutations_for_one_account_share_an_exclusive_lock() {
        let account = format!("https://lock-test-{}.invalid/", std::process::id());
        let path = mutation_lock_path(&account).unwrap();
        let first = acquire_mutation_lock(&account).unwrap();
        let second = open_mutation_lock(&account).unwrap();

        assert!(matches!(
            FileExt::try_lock(&second),
            Err(TryLockError::WouldBlock)
        ));

        drop(first);
        FileExt::lock(&second).unwrap();
        drop(second);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn credential_locks_use_the_private_application_data_directory() {
        let account = format!("https://lock-path-test-{}.invalid/", std::process::id());
        let path = mutation_lock_path(&account).unwrap();
        let directories = ProjectDirs::from("org", "Wekan", "wekan-cli").unwrap();

        assert!(path.starts_with(directories.data_local_dir()));
    }

    #[cfg(unix)]
    #[test]
    fn credential_lock_directory_is_owner_private() {
        use std::os::unix::fs::PermissionsExt;

        let account = format!(
            "https://lock-permissions-test-{}.invalid/",
            std::process::id()
        );
        let path = mutation_lock_path(&account).unwrap();
        let permissions = std::fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions();

        assert_eq!(permissions.mode() & 0o077, 0);
    }

    #[test]
    fn maps_unavailable_keyring_errors_to_unavailable_credentials() {
        for error in [
            keyring::Error::NoStorageAccess(Box::new(std::io::Error::other("locked"))),
            keyring::Error::PlatformFailure(Box::new(std::io::Error::other("unavailable"))),
            keyring::Error::NoDefaultStore,
            keyring::Error::NotSupportedByStore("read unavailable".to_owned()),
        ] {
            assert!(matches!(
                map_load_error(error),
                CredentialError::Unavailable(_)
            ));
        }
    }

    #[test]
    fn preserves_record_read_errors_as_load_failures() {
        assert!(matches!(
            map_load_error(keyring::Error::BadEncoding(vec![0xff])),
            CredentialError::Load(_)
        ));
    }

    #[test]
    fn deleting_an_absent_native_entry_is_idempotent() {
        assert!(!map_delete_error(keyring::Error::NoEntry).unwrap());
    }

    #[test]
    fn native_delete_failures_remain_delete_errors() {
        assert!(matches!(
            map_delete_error(keyring::Error::PlatformFailure(Box::new(
                std::io::Error::other("delete failed")
            ))),
            Err(CredentialError::Delete(_))
        ));
    }
}
