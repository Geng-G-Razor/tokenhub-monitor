use keyring::Entry;

const SERVICE: &str = "cash.tokenhub.monitor";
const KEY_USER: &str = "tokenhub-master-key";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("no stored key")]
    NotFound,
}

/// Store the master key in the OS credential store (macOS Keychain,
/// Windows Credential Manager, Linux Secret Service).
pub fn store_master_key(key: &str) -> Result<(), AuthError> {
    Entry::new(SERVICE, KEY_USER)?.set_password(key)?;
    Ok(())
}

/// Retrieve the master key from Keychain.
pub fn get_master_key() -> Result<String, AuthError> {
    Entry::new(SERVICE, KEY_USER)?
        .get_password()
        .map_err(|e| match e {
            keyring::Error::NoEntry => AuthError::NotFound,
            other => AuthError::Keyring(other),
        })
}

/// Delete the stored master key.
pub fn clear_master_key() -> Result<(), AuthError> {
    let _ = Entry::new(SERVICE, KEY_USER)?.delete_credential();
    Ok(())
}
