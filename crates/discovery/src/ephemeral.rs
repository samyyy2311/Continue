// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use rand::RngCore;
use std::time::Duration;

/// 24-hour lifetime for discovery identifiers.
pub const ROTATION_PERIOD: Duration = Duration::from_secs(24 * 60 * 60);

/// 128-bit random ephemeral identifier for discovery advertisements.
///
/// Replaced every 24 hours to prevent cross-network tracking.
/// Must never be set to the device identity fingerprint.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EphemeralDiscoveryId {
    bytes: [u8; 16],
}

impl EphemeralDiscoveryId {
    /// Generate a fresh random 128-bit ephemeral ID.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.bytes
    }

    /// The mDNS instance name.
    pub fn to_hex(&self) -> String {
        hex::encode(self.bytes)
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
    }
}
