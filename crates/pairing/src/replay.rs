// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::error::PairingError;
use limits::{MAX_PAIRING_CACHE_ENTRIES, MAX_PAIRING_SESSION_SECS};

struct CacheEntry {
    expires_at: Instant,
    consumed: bool,
}

/// The session tokens of codes on show, so each pairs at most once. Holds at most
/// `MAX_PAIRING_CACHE_ENTRIES`, each for `MAX_PAIRING_SESSION_SECS`.
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

    pub fn register(&self, token: [u8; 16]) -> Result<(), PairingError> {
        let mut map = self.entries.lock().unwrap();
        let now = Instant::now();
        map.retain(|_, v| v.expires_at > now);

        if map.contains_key(&token) {
            return Err(PairingError::SessionTokenReplayed);
        }

        if map.len() >= MAX_PAIRING_CACHE_ENTRIES {
            return Err(PairingError::InvalidQr(
                "Too many concurrent pairing sessions".to_string(),
            ));
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

    /// Uses up a token. Fails if it is unknown, expired or already used.
    pub fn consume(&self, token: &[u8; 16]) -> Result<(), PairingError> {
        let mut map = self.entries.lock().unwrap();
        let now = Instant::now();
        let expired = map
            .get(token)
            .ok_or(PairingError::SessionTokenMismatch)?
            .expires_at
            <= now;
        map.retain(|_, v| v.expires_at > now);
        if expired {
            return Err(PairingError::SessionTokenExpired);
        }

        let entry = map.get_mut(token).expect("checked above");
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

        assert!(matches!(
            cache.consume(&token),
            Err(PairingError::SessionTokenReplayed)
        ));
    }
}
