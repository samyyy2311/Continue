// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Device fingerprint: `Base64Url_NoPad(SHA-256(ed25519_public_key))`.
//!
//! The fingerprint is the stable, human-displayable identifier for a device.
//! It is 43 characters long and URL-safe.
//!
//! The fingerprint is used in the trust store and logs. Full fingerprints
//! are never logged by default â€” use `Fingerprint::diagnostic()` for log output.

use std::fmt;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::VerifyingKey;
use limits::{DEVICE_FINGERPRINT_STR_LEN, SHA256_DIGEST_LEN};
use sha2::{Digest, Sha256};

/// A device fingerprint: `Base64Url_NoPad(SHA-256(ed25519_public_key))`.
///
/// 43 characters, URL-safe, no padding.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Fingerprint(String);

impl Fingerprint {
    /// Compute a fingerprint from an Ed25519 verifying key.
    pub fn from_verifying_key(key: &VerifyingKey) -> Self {
        Self::from_pubkey_bytes(key.as_bytes())
    }

    /// Compute a fingerprint from raw Ed25519 public key bytes (32 bytes).
    pub fn from_pubkey_bytes(bytes: &[u8; 32]) -> Self {
        let digest = Sha256::digest(bytes);
        debug_assert_eq!(digest.len(), SHA256_DIGEST_LEN);
        let encoded = URL_SAFE_NO_PAD.encode(digest);
        debug_assert_eq!(encoded.len(), DEVICE_FINGERPRINT_STR_LEN);
        Self(encoded)
    }

    /// Parse a fingerprint from its string representation.
    ///
    /// Returns `None` if the input is not a valid 43-character Base64Url string
    /// that decodes to exactly 32 bytes.
    pub fn parse(s: &str) -> Option<Self> {
        if s.len() != DEVICE_FINGERPRINT_STR_LEN {
            return None;
        }
        let decoded = URL_SAFE_NO_PAD.decode(s).ok()?;
        if decoded.len() != SHA256_DIGEST_LEN {
            return None;
        }
        Some(Self(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// A redacted form for log output: first 8 characters followed by `â€¦`.
    ///
    /// Use this in all log fields that carry a peer reference. Full fingerprints
    /// are only shown in explicit trust-management UI.
    pub fn diagnostic(&self) -> &str {
        &self.0[..8]
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display uses the diagnostic form to prevent accidental full-fingerprint logging.
        write!(f, "{}â€¦", self.diagnostic())
    }
}

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Debug shows full fingerprint for development builds.
        write!(f, "Fingerprint({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key_bytes = [0u8; 32];
        let fp = Fingerprint::from_pubkey_bytes(&key_bytes);
        assert_eq!(fp.as_str().len(), DEVICE_FINGERPRINT_STR_LEN);
        let parsed = Fingerprint::parse(fp.as_str()).unwrap();
        assert_eq!(fp, parsed);
    }

    #[test]
    fn display_is_redacted() {
        let key_bytes = [0u8; 32];
        let fp = Fingerprint::from_pubkey_bytes(&key_bytes);
        let displayed = format!("{fp}");
        assert!(displayed.ends_with('â€¦'));
        assert_eq!(displayed.len(), 9); // 8 chars + ellipsis
    }

    #[test]
    fn parse_rejects_wrong_length() {
        assert!(Fingerprint::parse("tooshort").is_none());
        assert!(Fingerprint::parse(&"a".repeat(44)).is_none());
    }
}
