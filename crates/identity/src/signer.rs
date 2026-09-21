// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use ed25519_dalek::{Signature, SigningKey, VerifyingKey};

use crate::error::SignerError;

/// Signs messages with the device's persistent Ed25519 identity key.
///
/// Both methods are fallible because future implementations may delegate
/// to non-exportable hardware keys (e.g., Android Keystore API 33+, Secure Enclave)
/// that can fail due to hardware faults, OS policy, or key invalidation.
pub trait IdentitySigner: Send + Sync {
    fn verifying_key(&self) -> Result<VerifyingKey, SignerError>;
    fn sign(&self, message: &[u8]) -> Result<Signature, SignerError>;
}

/// In-memory Ed25519 signer backed by `ed25519-dalek`.
///
/// Used on all platforms in Phase 1. The signing key is loaded from the platform
/// secret store at startup and held in memory for the lifetime of the process.
/// The memory is zeroized on drop via `ed25519-dalek`'s `ZeroizeOnDrop` implementation.
pub struct InMemorySigner {
    key: SigningKey,
}

impl InMemorySigner {
    pub fn new(key: SigningKey) -> Self {
        Self { key }
    }
}

impl IdentitySigner for InMemorySigner {
    fn verifying_key(&self) -> Result<VerifyingKey, SignerError> {
        Ok(self.key.verifying_key())
    }

    fn sign(&self, message: &[u8]) -> Result<Signature, SignerError> {
        use ed25519_dalek::Signer;
        Ok(self.key.sign(message))
    }
}
