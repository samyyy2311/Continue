// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};
use rusqlite::{params, Connection, OptionalExtension};

use crate::error::PairingError;

/// A securely paired and trusted remote peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedPeer {
    pub fingerprint: String,
    pub identity_pubkey: [u8; 32],
    pub transport_spki_hash: [u8; 32],
    pub display_name: String,
    pub paired_at: u64,
}

/// SQLite-backed persistent trust store for paired peers.
#[derive(Clone)]
pub struct TrustStore {
    conn: Arc<Mutex<Connection>>,
}

impl TrustStore {
    pub fn new(conn: Connection) -> Result<Self, PairingError> {
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.init_schema()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self, PairingError> {
        let conn = Connection::open_in_memory()?;
        Self::new(conn)
    }

    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, PairingError> {
        let conn = Connection::open(path)?;
        Self::new(conn)
    }

    fn init_schema(&self) -> Result<(), PairingError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS trusted_peers (
                fingerprint         TEXT PRIMARY KEY,
                identity_pubkey     BLOB NOT NULL,
                transport_spki_hash BLOB NOT NULL,
                display_name        TEXT NOT NULL DEFAULT '',
                paired_at           INTEGER NOT NULL
            );",
        )?;
        Ok(())
    }

    pub fn add_peer(&self, peer: &TrustedPeer) -> Result<(), PairingError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO trusted_peers (fingerprint, identity_pubkey, transport_spki_hash, display_name, paired_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(fingerprint) DO UPDATE SET
                 identity_pubkey = excluded.identity_pubkey,
                 transport_spki_hash = excluded.transport_spki_hash,
                 display_name = excluded.display_name,
                 paired_at = excluded.paired_at;",
            params![
                peer.fingerprint,
                &peer.identity_pubkey[..],
                &peer.transport_spki_hash[..],
                peer.display_name,
                peer.paired_at as i64,
            ],
        )?;
        Ok(())
    }

    pub fn get_peer(&self, fingerprint: &str) -> Result<Option<TrustedPeer>, PairingError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT fingerprint, identity_pubkey, transport_spki_hash, display_name, paired_at
             FROM trusted_peers WHERE fingerprint = ?1;",
        )?;

        let peer = stmt
            .query_row(params![fingerprint], |row| {
                let fingerprint: String = row.get(0)?;
                let pubkey_raw: Vec<u8> = row.get(1)?;
                let spki_raw: Vec<u8> = row.get(2)?;
                let display_name: String = row.get(3)?;
                let paired_at: i64 = row.get(4)?;

                let mut identity_pubkey = [0u8; 32];
                let mut transport_spki_hash = [0u8; 32];
                if pubkey_raw.len() == 32 {
                    identity_pubkey.copy_from_slice(&pubkey_raw);
                }
                if spki_raw.len() == 32 {
                    transport_spki_hash.copy_from_slice(&spki_raw);
                }

                Ok(TrustedPeer {
                    fingerprint,
                    identity_pubkey,
                    transport_spki_hash,
                    display_name,
                    paired_at: paired_at as u64,
                })
            })
            .optional()?;

        Ok(peer)
    }

    pub fn list_peers(&self) -> Result<Vec<TrustedPeer>, PairingError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT fingerprint, identity_pubkey, transport_spki_hash, display_name, paired_at
             FROM trusted_peers ORDER BY paired_at DESC;",
        )?;

        let rows = stmt.query_map([], |row| {
            let fingerprint: String = row.get(0)?;
            let pubkey_raw: Vec<u8> = row.get(1)?;
            let spki_raw: Vec<u8> = row.get(2)?;
            let display_name: String = row.get(3)?;
            let paired_at: i64 = row.get(4)?;

            let mut identity_pubkey = [0u8; 32];
            let mut transport_spki_hash = [0u8; 32];
            if pubkey_raw.len() == 32 {
                identity_pubkey.copy_from_slice(&pubkey_raw);
            }
            if spki_raw.len() == 32 {
                transport_spki_hash.copy_from_slice(&spki_raw);
            }

            Ok(TrustedPeer {
                fingerprint,
                identity_pubkey,
                transport_spki_hash,
                display_name,
                paired_at: paired_at as u64,
            })
        })?;

        let mut peers = Vec::new();
        for peer in rows {
            peers.push(peer?);
        }
        Ok(peers)
    }

    pub fn remove_peer(&self, fingerprint: &str) -> Result<bool, PairingError> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM trusted_peers WHERE fingerprint = ?1;",
            params![fingerprint],
        )?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_store_crud() {
        let store = TrustStore::in_memory().unwrap();
        let peer = TrustedPeer {
            fingerprint: "test-fingerprint-1234".to_string(),
            identity_pubkey: [7u8; 32],
            transport_spki_hash: [9u8; 32],
            display_name: "My Phone".to_string(),
            paired_at: 1700000000,
        };

        store.add_peer(&peer).unwrap();

        let retrieved = store.get_peer(&peer.fingerprint).unwrap().expect("peer exists");
        assert_eq!(peer, retrieved);

        let list = store.list_peers().unwrap();
        assert_eq!(list.len(), 1);

        assert!(store.remove_peer(&peer.fingerprint).unwrap());
        assert!(store.get_peer(&peer.fingerprint).unwrap().is_none());
    }
}
