// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

mod error;
mod fingerprint;
mod signer;
mod store;

pub use error::{IdentityError, SecretStoreError, SignerError};
pub use fingerprint::Fingerprint;
pub use signer::{IdentitySigner, InMemorySigner};
pub use store::SecretStore;

use crypto::keys::{generate_ed25519_seed, signing_key_from_seed};
use tracing::debug;
use zeroize::Zeroizing;

/// Load or generate the device Ed25519 identity.
///
/// On first run, a new seed is generated and stored via `SecretStore`.
/// On subsequent runs, the existing seed is loaded, used to construct the keypair,
/// and then zeroized from memory.
///
/// Returns a boxed `IdentitySigner` ready for use.
pub fn load_or_create_identity(
    store: &dyn SecretStore,
    label: &str,
) -> Result<Box<dyn IdentitySigner>, IdentityError> {
    let seed = match store.load(label)? {
        Some(raw) => {
            if raw.len() != 32 {
                return Err(IdentityError::InvalidSeedLength(raw.len()));
            }
            let mut arr = Zeroizing::new([0u8; 32]);
            arr.copy_from_slice(&raw);
            arr
        }
        None => {
            debug!("No existing identity seed found; generating new Ed25519 identity");
            let fresh = generate_ed25519_seed();
            let bytes: Zeroizing<Vec<u8>> = Zeroizing::new(fresh.0.to_vec());
            store.store(label, bytes)?;
            fresh.0
        }
    };

    let signing_key = signing_key_from_seed(&seed);
    // Seed is zeroized when it drops at the end of this scope.
    Ok(Box::new(InMemorySigner::new(signing_key)))
}
