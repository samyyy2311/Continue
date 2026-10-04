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

    /// The folder the secrets are kept in.
    pub fn dir(&self) -> &Path {
        &self.base_dir
    }

    fn path_for(&self, label: &str) -> PathBuf {
        let safe_name: String = label
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.base_dir.join(format!("{safe_name}.secret"))
    }
}

impl SecretStore for FileSecretStore {
    fn store(&self, label: &str, secret: Zeroizing<Vec<u8>>) -> Result<(), SecretStoreError> {
        use std::io::Write;

        // Written whole beside the old one and then swapped in, so a crash never leaves half a
        // key; readable only by this user from the start, not after the fact.
        let file_path = self.path_for(label);
        let partial = file_path.with_extension("secret.partial");
        let io = |e: std::io::Error| SecretStoreError::Io(e.to_string());
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&partial).map_err(io)?;
        file.write_all(&secret).map_err(io)?;
        file.sync_all().map_err(io)?;
        drop(file);
        fs::rename(&partial, &file_path).map_err(io)
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

    #[cfg(unix)]
    #[test]
    fn secrets_are_only_readable_by_this_user() {
        use std::os::unix::fs::PermissionsExt;

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("continue_identity_mode_{nanos}"));
        let store = FileSecretStore::new(&temp_dir).unwrap();
        store.store("seed", Zeroizing::new(vec![1, 2, 3])).unwrap();
        store.store("seed", Zeroizing::new(vec![4, 5, 6])).unwrap();

        let mode = fs::metadata(temp_dir.join("seed.secret"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        assert_eq!(&*store.load("seed").unwrap().unwrap(), &[4, 5, 6]);
        assert_eq!(fs::read_dir(&temp_dir).unwrap().count(), 1, "no leftovers");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
