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
