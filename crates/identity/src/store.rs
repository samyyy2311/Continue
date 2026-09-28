// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use zeroize::Zeroizing;

use crate::error::SecretStoreError;

/// Platform-provided secret storage for small sensitive byte sequences.
///
/// Implementations must persist secrets to the platform's native secure store:
/// - Android: AES-256-GCM wrapping key in Android Keystore (see core-bridge)
/// - Windows: DPAPI
/// - macOS/iOS: Keychain
/// - Linux: D-Bus Secret Service or encrypted-file fallback
///
/// The `label` is an application-defined identifier for the stored item.
/// It must be stable across restarts and unique per secret.
pub trait SecretStore: Send + Sync {
    /// Persist `secret` under `label`.
    ///
    /// If a secret already exists under this label, it is overwritten.
    fn store(&self, label: &str, secret: Zeroizing<Vec<u8>>) -> Result<(), SecretStoreError>;

    /// Load the secret stored under `label`.
    ///
    /// Returns `Ok(None)` if no secret exists for this label.
    fn load(&self, label: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError>;

    /// Delete the secret stored under `label`.
    ///
    /// Returns `Ok(())` if the secret did not exist.
    fn delete(&self, label: &str) -> Result<(), SecretStoreError>;
}

use std::fs;
use std::path::{Path, PathBuf};

/// Simple file-based secret store that persists secrets in an application-managed directory.
pub struct FileSecretStore {
    base_dir: PathBuf,
}

impl FileSecretStore {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self, SecretStoreError> {
        let path = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&path).map_err(|e| SecretStoreError::Io(e.to_string()))?;
        Ok(Self { base_dir: path })
    }

    fn path_for(&self, label: &str) -> PathBuf {
        let safe_name: String = label
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        self.base_dir.join(format!("{safe_name}.secret"))
    }
}

impl SecretStore for FileSecretStore {
    fn store(&self, label: &str, secret: Zeroizing<Vec<u8>>) -> Result<(), SecretStoreError> {
        let file_path = self.path_for(label);
        fs::write(&file_path, &*secret).map_err(|e| SecretStoreError::Io(e.to_string()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(&file_path, perms);
        }

        Ok(())
    }

    fn load(&self, label: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
        let file_path = self.path_for(label);
        if !file_path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&file_path).map_err(|e| SecretStoreError::Io(e.to_string()))?;
        Ok(Some(Zeroizing::new(bytes)))
    }

    fn delete(&self, label: &str) -> Result<(), SecretStoreError> {
        let file_path = self.path_for(label);
        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| SecretStoreError::Io(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_secret_store_crud() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("continue_identity_test_{nanos}"));
        let store = FileSecretStore::new(&temp_dir).unwrap();

        assert_eq!(store.load("test_key").unwrap(), None);

        let secret = Zeroizing::new(vec![1, 2, 3, 4, 5]);
        store.store("test_key", secret).unwrap();

        let loaded = store.load("test_key").unwrap().unwrap();
        assert_eq!(&*loaded, &[1, 2, 3, 4, 5]);

        store.delete("test_key").unwrap();
        assert_eq!(store.load("test_key").unwrap(), None);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
