// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Pairing session token generation and expiry tracking.

use rand::{rngs::OsRng, RngCore};
use std::time::{Duration, Instant};

use limits::SESSION_TOKEN_LEN;

/// A single-use pairing session token with an expiry deadline.
///
/// The token is consumed atomically; a second call to `consume` always returns `false`.
pub struct SessionToken {
    bytes: [u8; SESSION_TOKEN_LEN],
    expires_at: Instant,
    consumed: bool,
}

impl SessionToken {
    /// Generate a fresh token that expires after `ttl`.
    pub fn new(ttl: Duration) -> Self {
        let mut bytes = [0u8; SESSION_TOKEN_LEN];
        OsRng.fill_bytes(&mut bytes);
        Self { bytes, expires_at: Instant::now() + ttl, consumed: false }
    }

    pub fn bytes(&self) -> &[u8; SESSION_TOKEN_LEN] {
        &self.bytes
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }

    /// Attempt to consume the token. Returns `true` exactly once if the token
    /// has not expired; returns `false` on any subsequent call or if expired.
    pub fn consume(&mut self) -> bool {
        if self.consumed || self.is_expired() {
            return false;
        }
        self.consumed = true;
        true
    }
}

/// Compare a received token against the expected token in constant time.
pub fn tokens_equal(a: &[u8; SESSION_TOKEN_LEN], b: &[u8; SESSION_TOKEN_LEN]) -> bool {
    use subtle::ConstantTimeEq;
    a.ct_eq(b).into()
}
