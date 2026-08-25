use keyring::Entry;

use super::{CredentialError, CredentialRecord, CredentialStore};

const SERVICE_NAME: &str = "wekan-cli";

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
        let entry = Entry::new(SERVICE_NAME, account).map_err(map_load_error)?;
        match entry.get_secret() {
            Ok(encoded) => CredentialRecord::decode(account, &encoded).map(Some),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(map_load_error(error)),
        }
    }

    fn save(&self, account: &str, record: &CredentialRecord) -> Result<(), CredentialError> {
        let entry = Entry::new(SERVICE_NAME, account)
            .map_err(|error| CredentialError::Store(error.to_string()))?;
        let encoded = record.encode()?;
        entry
            .set_secret(&encoded)
            .map_err(|error| CredentialError::Store(error.to_string()))
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
    use super::map_load_error;
    use crate::credentials::CredentialError;

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
}
