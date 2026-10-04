// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! The device keys, kept in the OS's secret store: the Keychain on macOS, Credential Manager
//! on Windows (protected with DPAPI), and the Secret Service on Linux.

use std::path::Path;

use identity::{FileSecretStore, PlatformFirstStore, SecretStore, SecretStoreError};
use zeroize::Zeroizing;

/// Groups this app's items in the OS store; matches the app identifier.
const SERVICE: &str = "org.continue.desktop";

struct OsSecretStore {
    service: String,
}

impl OsSecretStore {
    fn entry(&self, label: &str) -> Result<keyring::Entry, SecretStoreError> {
        keyring::Entry::new(&self.service, label).map_err(store_error)
    }
}

fn store_error(error: keyring::Error) -> SecretStoreError {
    match error {
        keyring::Error::NoStorageAccess(_) | keyring::Error::PlatformFailure(_) => {
            SecretStoreError::Unavailable(error.to_string())
        }
        other => SecretStoreError::Rejected(other.to_string()),
    }
}

impl SecretStore for OsSecretStore {
    fn store(&self, label: &str, secret: Zeroizing<Vec<u8>>) -> Result<(), SecretStoreError> {
        self.entry(label)?.set_secret(&secret).map_err(store_error)
    }

    fn load(&self, label: &str) -> Result<Option<Zeroizing<Vec<u8>>>, SecretStoreError> {
        match self.entry(label)?.get_secret() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(store_error(error)),
        }
    }

    fn delete(&self, label: &str) -> Result<(), SecretStoreError> {
        match self.entry(label)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(store_error(error)),
        }
    }
}

/// The store the device keys live in. Keys from earlier versions, kept in files in
/// `files_dir`, move into the OS store; where it can't be used, they stay in those files.
pub fn device_key_store(files_dir: &Path) -> Result<PlatformFirstStore, SecretStoreError> {
    store_for(SERVICE, files_dir)
}

fn store_for(service: &str, files_dir: &Path) -> Result<PlatformFirstStore, SecretStoreError> {
    Ok(PlatformFirstStore::new(
        Box::new(OsSecretStore {
            service: service.to_string(),
        }),
        FileSecretStore::new(files_dir)?,
    ))
}

#[cfg(test)]
mod tests {
    use pairing::DeviceKeys;

    use super::*;

    /// Needs a working OS store, e.g. on Linux an unlocked keyring on the session bus:
    /// `dbus-run-session -- sh -c 'echo | gnome-keyring-daemon --unlock && cargo test -- --ignored'`.
    /// Uses its own service name, so it never touches this machine's real Continue keys.
    #[test]
    #[ignore = "needs the OS secret store"]
    fn keys_in_files_move_into_the_os_store_and_stay_the_same() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let service = format!("{SERVICE}.test-{nanos}");
        let dir = std::env::temp_dir().join(format!("continue-os-store-{nanos}"));

        // As an earlier version left them.
        let before = DeviceKeys::load_or_create(&FileSecretStore::new(&dir).unwrap()).unwrap();
        let fingerprint = |keys: &DeviceKeys| keys.identity_signer.verifying_key().unwrap();

        let store = store_for(&service, &dir).unwrap();
        let moved = DeviceKeys::load_or_create(&store).unwrap();
        assert!(store.uses_platform(), "the OS store took the keys");
        assert_eq!(fingerprint(&moved), fingerprint(&before));
        assert_eq!(
            moved.transport_cert.spki_hash,
            before.transport_cert.spki_hash
        );
        let left: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| name.to_string_lossy().ends_with(".secret"))
            .collect();
        assert!(left.is_empty(), "no key left in files: {left:?}");

        // A restart reads them from the OS store.
        let again = DeviceKeys::load_or_create(&store_for(&service, &dir).unwrap()).unwrap();
        assert_eq!(fingerprint(&again), fingerprint(&before));
        assert_eq!(
            again.transport_cert.spki_hash,
            before.transport_cert.spki_hash
        );

        for label in ["device_identity", "transport_cert"] {
            store.delete(label).unwrap();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
