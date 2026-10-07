// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use protocol::CapabilityId;
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::PermissionError;

/// Per-device feature switches. Pairing is the trust, so everything is allowed unless turned off.
#[derive(Clone)]
pub struct PermissionStore {
    conn: Arc<Mutex<Connection>>,
}

impl PermissionStore {
    pub fn new(conn: Connection) -> Result<Self, PermissionError> {
        // Older versions also saved 'Ask', which now counts as on.
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
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn in_memory() -> Result<Self, PermissionError> {
        Self::new(Connection::open_in_memory()?)
    }

    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, PermissionError> {
        Self::new(Connection::open(path)?)
    }

    pub fn set_allowed(
        &self,
        peer_fingerprint: &str,
        capability: CapabilityId,
        allowed: bool,
    ) -> Result<(), PermissionError> {
        let updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs() as i64);
        self.conn.lock().unwrap().execute(
            "INSERT INTO peer_permissions (peer_fingerprint, capability_id, capability_version, grant, updated_at)
             VALUES (?1, ?2, 1, ?3, ?4)
             ON CONFLICT(peer_fingerprint, capability_id) DO UPDATE SET
                 grant = excluded.grant,
                 updated_at = excluded.updated_at;",
            params![
                peer_fingerprint,
                capability.raw(),
                if allowed { "Allow" } else { "Deny" },
                updated_at,
            ],
        )?;
        Ok(())
    }

    /// Unreadable rows count as denied.
    pub fn is_allowed(&self, peer_fingerprint: &str, capability: CapabilityId) -> bool {
        let grant: Result<Option<String>, _> = self
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT grant FROM peer_permissions WHERE peer_fingerprint = ?1 AND capability_id = ?2;",
                params![peer_fingerprint, capability.raw()],
                |row| row.get(0),
            )
            .optional();
        match grant {
            Ok(grant) => grant.as_deref() != Some("Deny"),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_is_on_until_turned_off() {
        let store = PermissionStore::in_memory().unwrap();
        let (peer, files) = ("device-fingerprint-abc", CapabilityId::FILE_TRANSFER);
        assert!(store.is_allowed(peer, files));

        store.set_allowed(peer, files, false).unwrap();
        assert!(!store.is_allowed(peer, files));
        assert!(store.is_allowed(peer, CapabilityId::CLIPBOARD));

        store.set_allowed(peer, files, true).unwrap();
        assert!(store.is_allowed(peer, files));
    }
}
