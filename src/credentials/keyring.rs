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

    fn save(&self, account: &str, record: &CredentialRecord) -> Result<(), CredentialError> {
        let entry = Entry::new(SERVICE_NAME, account)
            .map_err(|error| CredentialError::Store(error.to_string()))?;
        let encoded = record.encode()?;
        entry
            .set_secret(&encoded)
            .map_err(|error| CredentialError::Store(error.to_string()))
    }
}
