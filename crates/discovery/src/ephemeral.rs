// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, Instant};
use rand::RngCore;

/// 24-hour lifetime for discovery identifiers.
pub const ROTATION_PERIOD: Duration = Duration::from_secs(24 * 60 * 60);

/// 128-bit random ephemeral identifier for discovery advertisements.
///
/// Rotates every 24 hours to prevent cross-network tracking.
/// Must never be set to the device identity fingerprint.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EphemeralDiscoveryId {
    bytes: [u8; 16],
    created_at: Instant,
}

impl EphemeralDiscoveryId {
    /// Generate a fresh random 128-bit ephemeral ID.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self {
            bytes,
            created_at: Instant::now(),
        }
    }

    /// Construct from existing raw 16 bytes.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self {
            bytes,
            created_at: Instant::now(),
        }
    }

    /// Access raw 16 bytes.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.bytes
    }

    /// Check whether this ID has exceeded its 24-hour lifetime.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= ROTATION_PERIOD
    }

    /// Rotate the ID if it has expired, returning true if rotated.
    pub fn rotate_if_expired(&mut self) -> bool {
        if self.is_expired() {
            *self = Self::generate();
            true
        } else {
            false
        }
    }

    /// Convert to hex string for mDNS instance names and diagnostics.
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(32);
        for b in &self.bytes {
            use std::fmt::Write;
            let _ = write!(&mut s, "{:02x}", b);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_id_randomness_and_hex() {
        let id1 = EphemeralDiscoveryId::generate();
        let id2 = EphemeralDiscoveryId::generate();
        assert_ne!(id1.as_bytes(), id2.as_bytes());
        assert_eq!(id1.to_hex().len(), 32);
        assert!(!id1.is_expired());
    }
}
