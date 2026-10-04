// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Keeps secrets in the platform's secret store, moving them out of the older file store,
//! and falls back to the file where the platform has no usable store.

use tracing::warn;
use zeroize::Zeroizing;

use crate::error::SecretStoreError;
use crate::store::{FileSecretStore, SecretStore};

/// Written next to the file store, one per secret, once that secret is safely in the
/// platform store. From then on a platform error for it is reported instead of quietly
/// falling back to the file, where it no longer is: falling back would look like a first run
/// and replace the key, unpairing every device. Secrets the platform store hasn't taken yet
/// still fall back to the file.
const MOVED_MARKER: &str = "moved";

/// Prefers `platform`, with `file` as the store secrets are moved out of and the fallback
/// while the platform store hasn't worked yet.
pub struct PlatformFirstStore {
    platform: Box<dyn SecretStore>,
    file: FileSecretStore,
}

impl PlatformFirstStore {
    pub fn new(platform: Box<dyn SecretStore>, file: FileSecretStore) -> Self {
        Self { platform, file }
    }

    /// Whether `label` has been moved to the platform store, so it must be used from now on.
    pub fn uses_platform(&self, label: &str) -> bool {
        self.file.file_for(label, MOVED_MARKER).exists()
    }

    /// Stores `secret` in the platform store and reads it back. Only a secret that reads back
    /// the same counts as moved.
    fn store_in_platform(&self, label: &str, secret: &[u8]) -> Result<(), SecretStoreError> {
        self.platform
            .store(label, Zeroizing::new(secret.to_vec()))?;
        match self.platform.load(label)? {
            Some(read_back) if read_back.as_slice() == secret => {}
            _ => {
                return Err(SecretStoreError::Rejected(
                    "the platform store gave back something else".to_string(),
                ))
            }
        }
        std::fs::write(self.file.file_for(label, MOVED_MARKER), b"")
            .map_err(|e| SecretStoreError::Io(format!("couldn't note the move: {e}")))
    }

    /// Moves a secret found only in the file. On any failure it stays in the file.
    fn move_to_platform(&self, label: &str, secret: &[u8]) {
        match self.store_in_platform(label, secret) {
            Ok(()) => {
                if let Err(error) = self.file.delete(label) {
                    warn!("Moved {label} to the platform store but couldn't delete the file copy: {error}");
                }
            }
            Err(error) => warn!("Keeping {label} in a file for now: {error}"),
        }
    }
}

impl SecretStore for PlatformFirstStore {
    fn store(&self, label: &str, secret: Zeroizing<Vec<u8>>) -> Result<(), SecretStoreError> {
        match self.store_in_platform(label, &secret) {
            Ok(()) => {
                let _ = self.file.delete(label);
                Ok(())
            }
            Err(error) if !self.uses_platform(label) => {
                warn!("Saving {label} to a file, as the platform store failed: {error}");
                self.file.store(label, secret)
            }
            Err(error) => Err(error),
        }
    }

    fn load(&self, label: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
        match self.platform.load(label) {
            Ok(Some(secret)) => Ok(Some(secret)),
            // Not moved yet, or the platform store is new to this device.
            Ok(None) => {
                let found = self.file.load(label)?;
                if let Some(secret) = &found {
                    self.move_to_platform(label, secret);
                }
                Ok(found)
            }
            Err(error) if !self.uses_platform(label) => {
                warn!("Reading {label} from a file, as the platform store failed: {error}");
                self.file.load(label)
            }
            Err(error) => Err(error),
        }
    }

    fn delete(&self, label: &str) -> Result<(), SecretStoreError> {
        let from_platform = self.platform.delete(label);
        self.file.delete(label)?;
        match from_platform {
            Err(error) if self.uses_platform(label) => Err(error),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use super::*;

    /// A platform store that can be switched off, like a locked or missing keyring.
    #[derive(Clone, Default)]
    struct FakePlatform {
        secrets: Arc<Mutex<HashMap<String, Vec<u8>>>>,
        broken: Arc<AtomicBool>,
    }

    impl FakePlatform {
        fn check(&self) -> Result<(), SecretStoreError> {
            if self.broken.load(Ordering::Relaxed) {
                Err(SecretStoreError::Unavailable("locked".to_string()))
            } else {
                Ok(())
            }
        }

        fn has(&self, label: &str) -> bool {
            self.secrets.lock().unwrap().contains_key(label)
        }
    }

    impl SecretStore for FakePlatform {
        fn store(&self, label: &str, secret: Zeroizing<Vec<u8>>) -> Result<(), SecretStoreError> {
            self.check()?;
            self.secrets
                .lock()
                .unwrap()
                .insert(label.to_string(), secret.to_vec());
            Ok(())
        }

        fn load(&self, label: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
            self.check()?;
            Ok(self
                .secrets
                .lock()
                .unwrap()
                .get(label)
                .map(|s| Zeroizing::new(s.clone())))
        }

        fn delete(&self, label: &str) -> Result<(), SecretStoreError> {
            self.check()?;
            self.secrets.lock().unwrap().remove(label);
            Ok(())
        }
    }

    fn dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("continue_platform_first_{nanos}"))
    }

    fn stores(dir: &PathBuf, platform: &FakePlatform) -> PlatformFirstStore {
        PlatformFirstStore::new(
            Box::new(platform.clone()),
            FileSecretStore::new(dir).unwrap(),
        )
    }

    fn secret(bytes: &[u8]) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(bytes.to_vec())
    }

    #[test]
    fn a_secret_in_a_file_moves_to_the_platform_store() {
        let dir = dir();
        FileSecretStore::new(&dir)
            .unwrap()
            .store("seed", secret(b"old"))
            .unwrap();
        let platform = FakePlatform::default();
        let store = stores(&dir, &platform);

        assert_eq!(store.load("seed").unwrap().unwrap().as_slice(), b"old");
        assert!(platform.has("seed"));
        assert!(store.uses_platform("seed"));
        assert_eq!(
            FileSecretStore::new(&dir).unwrap().load("seed").unwrap(),
            None
        );
        assert_eq!(store.load("seed").unwrap().unwrap().as_slice(), b"old");
    }

    #[test]
    fn without_a_platform_store_the_file_keeps_working() {
        let dir = dir();
        let platform = FakePlatform::default();
        platform.broken.store(true, Ordering::Relaxed);
        let store = stores(&dir, &platform);

        assert_eq!(store.load("seed").unwrap(), None);
        store.store("seed", secret(b"new")).unwrap();
        assert_eq!(store.load("seed").unwrap().unwrap().as_slice(), b"new");
        assert!(!store.uses_platform("seed"));

        // Once the platform store works, the secret moves there.
        platform.broken.store(false, Ordering::Relaxed);
        assert_eq!(store.load("seed").unwrap().unwrap().as_slice(), b"new");
        assert!(platform.has("seed") && store.uses_platform("seed"));
    }

    #[test]
    fn a_failing_platform_store_after_the_move_is_an_error_not_a_first_run() {
        let dir = dir();
        let platform = FakePlatform::default();
        let store = stores(&dir, &platform);
        store.store("seed", secret(b"key")).unwrap();

        platform.broken.store(true, Ordering::Relaxed);
        assert!(
            store.load("seed").is_err(),
            "must not look like there's no key"
        );
        assert!(store.store("seed", secret(b"replacement")).is_err());

        platform.broken.store(false, Ordering::Relaxed);
        assert_eq!(store.load("seed").unwrap().unwrap().as_slice(), b"key");
    }

    #[test]
    fn a_secret_that_reads_back_differently_stays_in_the_file() {
        /// Saves anything but always gives back the same wrong bytes.
        struct Garbling;
        impl SecretStore for Garbling {
            fn store(&self, _: &str, _: Zeroizing<Vec<u8>>) -> Result<(), SecretStoreError> {
                Ok(())
            }
            fn load(&self, _: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
                Ok(Some(Zeroizing::new(b"garbled".to_vec())))
            }
            fn delete(&self, _: &str) -> Result<(), SecretStoreError> {
                Ok(())
            }
        }
        let dir = dir();
        let store =
            PlatformFirstStore::new(Box::new(Garbling), FileSecretStore::new(&dir).unwrap());

        store.store("seed", secret(b"key")).unwrap();
        assert!(!store.uses_platform("seed"));
        assert_eq!(
            FileSecretStore::new(&dir)
                .unwrap()
                .load("seed")
                .unwrap()
                .unwrap()
                .as_slice(),
            b"key"
        );
    }

    #[test]
    fn a_secret_the_platform_store_refuses_falls_back_even_after_another_moved() {
        /// Takes the seed but refuses anything else, like a keyring with a size limit.
        #[derive(Clone, Default)]
        struct Picky(FakePlatform);
        impl SecretStore for Picky {
            fn store(
                &self,
                label: &str,
                secret: Zeroizing<Vec<u8>>,
            ) -> Result<(), SecretStoreError> {
                if label != "seed" {
                    return Err(SecretStoreError::Rejected("too long".to_string()));
                }
                self.0.store(label, secret)
            }
            fn load(&self, label: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
                self.0.load(label)
            }
            fn delete(&self, label: &str) -> Result<(), SecretStoreError> {
                self.0.delete(label)
            }
        }
        let dir = dir();
        let store = PlatformFirstStore::new(
            Box::new(Picky::default()),
            FileSecretStore::new(&dir).unwrap(),
        );

        store.store("seed", secret(b"key")).unwrap();
        store.store("cert", secret(b"pem")).unwrap();
        assert!(store.uses_platform("seed") && !store.uses_platform("cert"));
        assert_eq!(store.load("cert").unwrap().unwrap().as_slice(), b"pem");
    }

    #[test]
    fn delete_removes_both_copies() {
        let dir = dir();
        let platform = FakePlatform::default();
        let store = stores(&dir, &platform);
        store.store("seed", secret(b"key")).unwrap();
        FileSecretStore::new(&dir)
            .unwrap()
            .store("seed", secret(b"stale"))
            .unwrap();

        store.delete("seed").unwrap();
        assert!(!platform.has("seed"));
        assert_eq!(
            FileSecretStore::new(&dir).unwrap().load("seed").unwrap(),
            None
        );
    }
}
