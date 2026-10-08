// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use protocol::CapabilityId;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::PermissionError;
use crate::types::{PermissionState, PersistedGrant};

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Saved per-peer grants, plus one-use grants that live only in memory.
#[derive(Clone)]
pub struct PermissionStore {
    conn: Arc<Mutex<Connection>>,
    allow_once_grants: Arc<Mutex<HashSet<(String, u32)>>>,
    lockdown: Arc<AtomicBool>,
}

impl PermissionStore {
    pub fn new(conn: Connection) -> Result<Self, PermissionError> {
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            allow_once_grants: Arc::new(Mutex::new(HashSet::new())),
            lockdown: Arc::new(AtomicBool::new(false)),
        };
        store.init_schema()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self, PermissionError> {
        let conn = Connection::open_in_memory()?;
        Self::new(conn)
    }

    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, PermissionError> {
        let conn = Connection::open(path)?;
        Self::new(conn)
    }

    fn init_schema(&self) -> Result<(), PermissionError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS peer_permissions (
                peer_fingerprint    TEXT NOT NULL,
                capability_id       INTEGER NOT NULL,
                capability_version  INTEGER NOT NULL,
                grant               TEXT NOT NULL CHECK (grant IN ('Allow', 'Deny', 'Ask')),
                updated_at          INTEGER NOT NULL,
                PRIMARY KEY (peer_fingerprint, capability_id)
            );",
        )?;
        Ok(())
    }

    /// Sets what `peer_fingerprint` may do with `capability`. `AllowOnce` lasts until it is
    /// used or the session ends; the others are saved.
    pub fn set_state(
        &self,
        peer_fingerprint: &str,
        capability: CapabilityId,
        state: PermissionState,
    ) -> Result<(), PermissionError> {
        let grant = match state {
            PermissionState::AllowOnce => {
                self.grant_allow_once(peer_fingerprint, capability);
                return Ok(());
            }
            PermissionState::Allow => PersistedGrant::Allow,
            PermissionState::Deny => PersistedGrant::Deny,
            PermissionState::Ask => PersistedGrant::Ask,
        };
        self.set_persisted_grant(peer_fingerprint, capability, 1, grant)
    }

    pub fn set_persisted_grant(
        &self,
        peer_fingerprint: &str,
        capability: CapabilityId,
        version: u32,
        grant: PersistedGrant,
    ) -> Result<(), PermissionError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO peer_permissions (peer_fingerprint, capability_id, capability_version, grant, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(peer_fingerprint, capability_id) DO UPDATE SET
                 capability_version = excluded.capability_version,
                 grant = excluded.grant,
                 updated_at = excluded.updated_at;",
            params![
                peer_fingerprint,
                capability.raw(),
                version,
                grant.as_str(),
                current_timestamp() as i64,
            ],
        )?;
        Ok(())
    }

    pub fn get_persisted_grant(
        &self,
        peer_fingerprint: &str,
        capability: CapabilityId,
    ) -> Result<Option<PersistedGrant>, PermissionError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT grant FROM peer_permissions WHERE peer_fingerprint = ?1 AND capability_id = ?2;",
        )?;

        let grant_str: Option<String> = stmt
            .query_row(params![peer_fingerprint, capability.raw()], |row| {
                row.get(0)
            })
            .optional()?;

        match grant_str {
            Some(s) => PersistedGrant::parse(&s)
                .map(Some)
                .ok_or_else(|| PermissionError::InvalidGrantString(s)),
            None => Ok(None),
        }
    }

    /// Allows one use, kept in memory only.
    pub fn grant_allow_once(&self, peer_fingerprint: &str, capability: CapabilityId) {
        let mut set = self.allow_once_grants.lock().unwrap();
        set.insert((peer_fingerprint.to_string(), capability.raw()));
    }

    /// Drops the peer's one-use grants, when its session ends.
    pub fn clear_allow_once_for_peer(&self, peer_fingerprint: &str) {
        let mut set = self.allow_once_grants.lock().unwrap();
        set.retain(|(peer, _)| peer != peer_fingerprint);
    }

    /// Enables or disables global lockdown mode.
    /// When active, all permission queries evaluate to `Deny` regardless of stored grants.
    pub fn set_lockdown(&self, active: bool) {
        self.lockdown.store(active, Ordering::SeqCst);
    }

    /// Returns true if global lockdown mode is currently active.
    pub fn is_locked_down(&self) -> bool {
        self.lockdown.load(Ordering::SeqCst)
    }

    /// Uses up a one-use grant, if there is one.
    pub fn consume_if_allow_once(&self, peer_fingerprint: &str, capability: CapabilityId) -> bool {
        if self.is_locked_down() {
            return false;
        }
        let mut set = self.allow_once_grants.lock().unwrap();
        set.remove(&(peer_fingerprint.to_string(), capability.raw()))
    }

    /// What the peer may do now: Deny if lockdown is active, a one-use grant first, then the saved grant, else `Ask`.
    pub fn query_state(
        &self,
        peer_fingerprint: &str,
        capability: CapabilityId,
    ) -> Result<PermissionState, PermissionError> {
        if self.is_locked_down() {
            return Ok(PermissionState::Deny);
        }
        let allowed_once = self
            .allow_once_grants
            .lock()
            .unwrap()
            .contains(&(peer_fingerprint.to_string(), capability.raw()));
        if allowed_once {
            return Ok(PermissionState::AllowOnce);
        }
        Ok(self
            .get_persisted_grant(peer_fingerprint, capability)?
            .map_or(PermissionState::Ask, PermissionState::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_once_is_in_memory_only_and_clears() {
        let store = PermissionStore::in_memory().unwrap();
        let peer = "device-fingerprint-abc";
        let cap = CapabilityId::FILE_TRANSFER;

        assert_eq!(store.query_state(peer, cap).unwrap(), PermissionState::Ask);

        store.grant_allow_once(peer, cap);
        assert_eq!(
            store.query_state(peer, cap).unwrap(),
            PermissionState::AllowOnce
        );

        assert_eq!(store.get_persisted_grant(peer, cap).unwrap(), None);

        assert!(store.consume_if_allow_once(peer, cap));
        assert!(!store.consume_if_allow_once(peer, cap));
        assert_eq!(store.query_state(peer, cap).unwrap(), PermissionState::Ask);

        store.grant_allow_once(peer, cap);

        store.clear_allow_once_for_peer(peer);
        assert_eq!(store.query_state(peer, cap).unwrap(), PermissionState::Ask);

        store
            .set_persisted_grant(peer, cap, 1, PersistedGrant::Allow)
            .unwrap();
        assert_eq!(
            store.query_state(peer, cap).unwrap(),
            PermissionState::Allow
        );
    }

    #[test]
    fn lockdown_overrides_grants_and_resets() {
        let store = PermissionStore::in_memory().unwrap();
        let peer = "device-fingerprint-xyz";
        let cap = CapabilityId::FILE_TRANSFER;

        store
            .set_persisted_grant(peer, cap, 1, PersistedGrant::Allow)
            .unwrap();
        assert_eq!(
            store.query_state(peer, cap).unwrap(),
            PermissionState::Allow
        );

        store.grant_allow_once(peer, CapabilityId::CLIPBOARD);
        assert_eq!(
            store.query_state(peer, CapabilityId::CLIPBOARD).unwrap(),
            PermissionState::AllowOnce
        );

        store.set_lockdown(true);
        assert!(store.is_locked_down());

        assert_eq!(store.query_state(peer, cap).unwrap(), PermissionState::Deny);
        assert_eq!(
            store.query_state(peer, CapabilityId::CLIPBOARD).unwrap(),
            PermissionState::Deny
        );
        assert!(!store.consume_if_allow_once(peer, CapabilityId::CLIPBOARD));

        // Shared across clones
        let store_clone = store.clone();
        assert!(store_clone.is_locked_down());
        assert_eq!(
            store_clone.query_state(peer, cap).unwrap(),
            PermissionState::Deny
        );

        // Deactivating lockdown restores configured grants
        store.set_lockdown(false);
        assert!(!store.is_locked_down());
        assert_eq!(
            store.query_state(peer, cap).unwrap(),
            PermissionState::Allow
        );
        assert_eq!(
            store.query_state(peer, CapabilityId::CLIPBOARD).unwrap(),
            PermissionState::AllowOnce
        );
        assert!(store.consume_if_allow_once(peer, CapabilityId::CLIPBOARD));
    }
}
