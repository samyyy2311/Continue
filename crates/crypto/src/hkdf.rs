// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! HKDF-based key derivation for pairing, session, and confirmation keys.

use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::error::CryptoError;

/// Labels used as HKDF `info` inputs. Labels are domain-separated and version-tagged.
/// Using a distinct constant for each key purpose prevents key confusion.
pub const INFO_CONFIRMATION_KEY: &[u8] = b"CONFIRM_KEY_V1";
pub const INFO_SESSION_KEY: &[u8] = b"SESSION_KEY_V1";
pub const INFO_RELAY_A_TO_B: &[u8] = b"RELAY_A_TO_B_V1";
pub const INFO_RELAY_B_TO_A: &[u8] = b"RELAY_B_TO_A_V1";

/// Derive a 32-byte key from an ECDH output and a salt.
///
/// `ikm` is the raw X25519 shared secret; `salt` is the pairing session token;
/// `info` is a label constant from this module.
pub fn derive_key(
    ikm: &[u8],
    salt: &[u8],
    info: &[u8],
) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut out = Zeroizing::new([0u8; 32]);
    hk.expand(info, out.as_mut())
        .map_err(|_| CryptoError::HkdfLength)?;
    Ok(out)
}

/// Derive both the confirmation key and session key in one pass.
///
/// Returns `(confirmation_key, session_key)`. The session key is a pairing-session
/// key used only for PairConfirm. It is discarded after pairing and never reused
/// for relay or data-transfer sessions.
pub fn derive_pairing_keys(
    dh_output: &Zeroizing<[u8; 32]>,
    session_token: &[u8],
) -> Result<(Zeroizing<[u8; 32]>, Zeroizing<[u8; 32]>), CryptoError> {
    let confirmation_key = derive_key(dh_output.as_ref(), session_token, INFO_CONFIRMATION_KEY)?;
    let session_key = derive_key(dh_output.as_ref(), session_token, INFO_SESSION_KEY)?;
    Ok((confirmation_key, session_key))
}
