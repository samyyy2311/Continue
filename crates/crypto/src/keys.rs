// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Ed25519 and X25519 key types and operations.

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};
use zeroize::Zeroizing;

use crate::error::CryptoError;

/// A freshly generated Ed25519 signing keypair with its raw seed.
///
/// The seed is wrapped in `Zeroizing` and must be stored encrypted at rest
/// via the platform `SecretStore`.
pub struct Ed25519Seed(pub Zeroizing<[u8; 32]>);

/// Generate a new Ed25519 seed. The returned seed must be persisted via the platform
/// `SecretStore` before the `SigningKey` is constructed.
pub fn generate_ed25519_seed() -> Ed25519Seed {
    let mut bytes = Zeroizing::new([0u8; 32]);
    // OsRng is a CSPRNG backed by the OS entropy source.
    use rand::RngCore;
    OsRng.fill_bytes(bytes.as_mut());
    Ed25519Seed(bytes)
}

/// Construct an `Ed25519SigningKey` from a 32-byte seed.
///
/// The seed is consumed and zeroized on drop. Callers must zeroize any
/// intermediate buffer that held the seed bytes before calling this function.
pub fn signing_key_from_seed(seed: &Zeroizing<[u8; 32]>) -> SigningKey {
    SigningKey::from_bytes(seed)
}

/// Extract the verifying (public) key from a signing key.
pub fn verifying_key(signing: &SigningKey) -> VerifyingKey {
    signing.verifying_key()
}

/// An ephemeral X25519 keypair. The secret is consumed on ECDH.
pub struct EphemeralX25519(EphemeralSecret);

impl EphemeralX25519 {
    pub fn generate() -> Self {
        Self(EphemeralSecret::random_from_rng(OsRng))
    }

    pub fn public_key(&self) -> X25519PublicKey {
        X25519PublicKey::from(&self.0)
    }

    /// Consume the secret and produce the DH output.
    ///
    /// The result is the raw 32-byte shared secret. Pass it through HKDF before use.
    pub fn diffie_hellman(self, peer_public: &X25519PublicKey) -> Zeroizing<[u8; 32]> {
        let shared = self.0.diffie_hellman(peer_public);
        Zeroizing::new(*shared.as_bytes())
    }
}

/// Verify an Ed25519 signature over `message` using `verifying_key`.
pub fn verify_ed25519(
    verifying_key: &VerifyingKey,
    message: &[u8],
    signature_bytes: &[u8; 64],
) -> Result<(), CryptoError> {
    use ed25519_dalek::{Signature, Verifier};
    let sig = Signature::from_bytes(signature_bytes);
    verifying_key
        .verify(message, &sig)
        .map_err(|_| CryptoError::SignatureInvalid)
}
