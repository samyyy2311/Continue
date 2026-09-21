// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! ChaCha20-Poly1305 AEAD for future relay inner-E2EE sessions.
//!
//! Not used for direct peer-to-peer connections (which rely on QUIC TLS 1.3).

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use zeroize::Zeroizing;

use crate::error::CryptoError;

/// Encrypt `plaintext` with a ChaCha20-Poly1305 key and 12-byte nonce.
///
/// Returns `nonce || ciphertext_with_tag`.
pub fn encrypt(
    key: &Zeroizing<[u8; 32]>,
    nonce: &[u8; 12],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key.as_ref()));
    let nonce = Nonce::from_slice(nonce);
    let mut payload = chacha20poly1305::aead::Payload { msg: plaintext, aad };
    cipher.encrypt(nonce, payload).map_err(|_| CryptoError::AeadEncrypt)
}

/// Decrypt ciphertext (with appended authentication tag) produced by `encrypt`.
pub fn decrypt(
    key: &Zeroizing<[u8; 32]>,
    nonce: &[u8; 12],
    ciphertext_with_tag: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key.as_ref()));
    let nonce = Nonce::from_slice(nonce);
    let payload = chacha20poly1305::aead::Payload { msg: ciphertext_with_tag, aad };
    cipher.decrypt(nonce, payload).map_err(|_| CryptoError::AeadDecrypt)
}
