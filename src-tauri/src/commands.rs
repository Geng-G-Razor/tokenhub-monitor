use crate::api::{self, ApiError, PackageData};
use crate::auth;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    Api(#[from] ApiError),
    #[error("auth: {0}")]
    Auth(#[from] auth::AuthError),
    #[error("no master key")]
    NoMasterKey,
}

impl serde::Serialize for CommandError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.to_string().as_ref())
    }
}

type CmdResult<T> = std::result::Result<T, CommandError>;

/// Save a master key to Keychain.
#[tauri::command]
pub fn save_master_key(key: String) -> CmdResult<bool> {
    auth::store_master_key(&key)?;
    Ok(true)
}

/// Clear the stored master key (logout).
#[tauri::command]
pub fn clear_master_key() -> CmdResult<bool> {
    auth::clear_master_key()?;
    Ok(true)
}

/// Whether a master key is stored.
#[tauri::command]
pub fn has_master_key() -> bool {
    auth::get_master_key().is_ok()
}

/// Fetch the package data.
#[tauri::command]
pub async fn fetch_package() -> CmdResult<PackageData> {
    let key = auth::get_master_key().map_err(|_| CommandError::NoMasterKey)?;
    Ok(api::fetch_package(&key).await?)
}
