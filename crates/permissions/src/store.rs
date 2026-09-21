// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
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

/// Persistent SQLite permission storage combined with ephemeral in-memory AllowOnce grants.
#[derive(Clone)]
pub struct PermissionStore {
    conn: Arc<Mutex<Connection>>,
    allow_once_grants: Arc<Mutex<HashSet<(String, u32)>>>,
}

impl PermissionStore {
    pub fn new(conn: Connection) -> Result<Self, PermissionError> {
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
            allow_once_grants: Arc::new(Mutex::new(HashSet::new())),
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

    /// Set a persistent permission grant for a peer and capability.
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

    /// Get the persistent grant for a peer and capability, if configured in the database.
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
            .query_row(params![peer_fingerprint, capability.raw()], |row| row.get(0))
            .optional()?;

        match grant_str {
            Some(s) => PersistedGrant::parse(&s)
                .map(Some)
                .ok_or_else(|| PermissionError::InvalidGrantString(s)),
            None => Ok(None),
        }
    }

    /// Grant a single-use permission in-memory only.
    ///
    /// Never written to the SQLite database.
    pub fn grant_allow_once(&self, peer_fingerprint: &str, capability: CapabilityId) {
        let mut set = self.allow_once_grants.lock().unwrap();
        set.insert((peer_fingerprint.to_string(), capability.raw()));
    }

    /// Clear all ephemeral AllowOnce grants for a peer (called on session termination).
    pub fn clear_allow_once_for_peer(&self, peer_fingerprint: &str) {
        let mut set = self.allow_once_grants.lock().unwrap();
        set.retain(|(peer, _)| peer != peer_fingerprint);
    }

    /// Query the effective runtime permission state.
    pub fn query_state(
        &self,
        peer_fingerprint: &str,
        capability: CapabilityId,
    ) -> Result<PermissionState, PermissionError> {
        // 1. Check in-memory AllowOnce grants first
        {
            let set = self.allow_once_grants.lock().unwrap();
            if set.contains(&(peer_fingerprint.to_string(), capability.raw())) {
                return Ok(PermissionState::AllowOnce);
            }
        }

        // 2. Query persistent SQLite database
        match self.get_persisted_grant(peer_fingerprint, capability)? {
            Some(persisted) => Ok(persisted.into()),
            None => Ok(PermissionState::Ask), // default when unconfigured
        }
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

        // Default is Ask
        assert_eq!(store.query_state(peer, cap).unwrap(), PermissionState::Ask);

        // Grant AllowOnce
        store.grant_allow_once(peer, cap);
        assert_eq!(store.query_state(peer, cap).unwrap(), PermissionState::AllowOnce);

        // Verify DB still has NO row
        assert_eq!(store.get_persisted_grant(peer, cap).unwrap(), None);

        // Session drops -> clear in-memory grants
        store.clear_allow_once_for_peer(peer);
        assert_eq!(store.query_state(peer, cap).unwrap(), PermissionState::Ask);

        // Persist Allow
        store.set_persisted_grant(peer, cap, 1, PersistedGrant::Allow).unwrap();
        assert_eq!(store.query_state(peer, cap).unwrap(), PermissionState::Allow);
    }
}
