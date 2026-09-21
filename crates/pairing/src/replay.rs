// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use limits::{MAX_PAIRING_CACHE_ENTRIES, MAX_PAIRING_SESSION_SECS};
use crate::error::PairingError;

struct CacheEntry {
    expires_at: Instant,
    consumed: bool,
}

/// In-memory bounded replay cache for pairing session tokens.
///
/// Prevents token reuse and caps concurrent pairing session memory to 64 entries.
pub struct ReplayCache {
    entries: Mutex<HashMap<[u8; 16], CacheEntry>>,
}

impl Default for ReplayCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::with_capacity(MAX_PAIRING_CACHE_ENTRIES)),
        }
    }

    /// Register a newly generated session token with a 60-second TTL.
    pub fn register(&self, token: [u8; 16]) -> Result<(), PairingError> {
        let mut map = self.entries.lock().unwrap();
        let now = Instant::now();

        // Prune expired entries
        map.retain(|_, v| v.expires_at > now);

        if map.contains_key(&token) {
            return Err(PairingError::SessionTokenReplayed);
        }

        if map.len() >= MAX_PAIRING_CACHE_ENTRIES {
            return Err(PairingError::InvalidQr("Too many concurrent pairing sessions".to_string()));
        }

        map.insert(
            token,
            CacheEntry {
                expires_at: now + Duration::from_secs(MAX_PAIRING_SESSION_SECS),
                consumed: false,
            },
        );

        Ok(())
    }

    /// Atomically consume a token. Returns an error if expired, replayed, or unknown.
    pub fn consume(&self, token: &[u8; 16]) -> Result<(), PairingError> {
        let mut map = self.entries.lock().unwrap();
        let now = Instant::now();

        // Prune expired entries
        map.retain(|_, v| v.expires_at > now);

        let entry = map.get_mut(token).ok_or(PairingError::SessionTokenMismatch)?;

        if entry.expires_at <= now {
            return Err(PairingError::SessionTokenExpired);
        }

        if entry.consumed {
            return Err(PairingError::SessionTokenReplayed);
        }

        entry.consumed = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_use_and_replay_rejection() {
        let cache = ReplayCache::new();
        let token = [1u8; 16];

        cache.register(token).unwrap();
        assert!(cache.consume(&token).is_ok());

        // Second consume must fail as replayed
        assert!(matches!(
            cache.consume(&token),
            Err(PairingError::SessionTokenReplayed)
        ));
    }
}
