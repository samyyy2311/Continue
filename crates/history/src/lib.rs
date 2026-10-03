// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! What this device sent and received, kept across restarts.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use thiserror::Error;

/// Older entries beyond this are dropped as new ones come in.
pub const HISTORY_LIMIT: u32 = 500;

/// Received text longer than this is kept cut short.
const TEXT_LIMIT: usize = 4_000;

#[derive(Debug, Error)]
#[error("History database error: {0}")]
pub struct HistoryError(#[from] rusqlite::Error);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Sent,
    Received,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    Text,
}

/// One file or piece of text that went between devices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub direction: Direction,
    pub kind: Kind,
    /// The file name, or the text itself.
    pub label: String,
    pub peer_fingerprint: String,
    pub peer_name: String,
    pub size: u64,
    pub failed: bool,
    /// Where the file is on this device, when known.
    pub location: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: i64,
    /// Unix time in milliseconds.
    pub at: u64,
    pub item: Item,
}

#[derive(Clone)]
pub struct HistoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl HistoryStore {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, HistoryError> {
        Self::new(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self, HistoryError> {
        Self::new(Connection::open_in_memory()?)
    }

    fn new(conn: Connection) -> Result<Self, HistoryError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id                INTEGER PRIMARY KEY AUTOINCREMENT,
                at                INTEGER NOT NULL,
                received          INTEGER NOT NULL,
                is_text           INTEGER NOT NULL,
                label             TEXT NOT NULL,
                peer_fingerprint  TEXT NOT NULL,
                peer_name         TEXT NOT NULL,
                size              INTEGER NOT NULL,
                failed            INTEGER NOT NULL,
                location          TEXT
            );",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Saves an item and returns its id.
    pub fn record(&self, item: &Item) -> Result<i64, HistoryError> {
        let label: String = match item.kind {
            Kind::Text => item.label.chars().take(TEXT_LIMIT).collect(),
            Kind::File => item.label.clone(),
        };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO history
                (at, received, is_text, label, peer_fingerprint, peer_name, size, failed, location)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9);",
            params![
                now_ms() as i64,
                item.direction == Direction::Received,
                item.kind == Kind::Text,
                label,
                item.peer_fingerprint,
                item.peer_name,
                item.size as i64,
                item.failed,
                item.location,
            ],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "DELETE FROM history WHERE id <= ?1;",
            params![id - i64::from(HISTORY_LIMIT)],
        )?;
        Ok(id)
    }

    /// Notes where a received file ended up after the app moved it.
    pub fn set_location(&self, id: i64, location: &str) -> Result<(), HistoryError> {
        self.conn.lock().unwrap().execute(
            "UPDATE history SET location = ?2 WHERE id = ?1;",
            params![id, location],
        )?;
        Ok(())
    }

    /// The newest `limit` entries, newest first.
    pub fn list(&self, limit: u32) -> Result<Vec<Entry>, HistoryError> {
        let conn = self.conn.lock().unwrap();
        let mut statement = conn.prepare(
            "SELECT id, at, received, is_text, label, peer_fingerprint, peer_name, size, failed, location
             FROM history ORDER BY id DESC LIMIT ?1;",
        )?;
        let entries = statement
            .query_map(params![limit], |row| {
                Ok(Entry {
                    id: row.get(0)?,
                    at: row.get::<_, i64>(1)? as u64,
                    item: Item {
                        direction: if row.get(2)? {
                            Direction::Received
                        } else {
                            Direction::Sent
                        },
                        kind: if row.get(3)? { Kind::Text } else { Kind::File },
                        label: row.get(4)?,
                        peer_fingerprint: row.get(5)?,
                        peer_name: row.get(6)?,
                        size: row.get::<_, i64>(7)? as u64,
                        failed: row.get(8)?,
                        location: row.get(9)?,
                    },
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(entries)
    }

    pub fn clear(&self) -> Result<(), HistoryError> {
        self.conn
            .lock()
            .unwrap()
            .execute("DELETE FROM history;", [])?;
        Ok(())
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent_file(name: &str) -> Item {
        Item {
            direction: Direction::Sent,
            kind: Kind::File,
            label: name.to_string(),
            peer_fingerprint: "phone".to_string(),
            peer_name: "Pixel 8".to_string(),
            size: 10,
            failed: false,
            location: Some(format!("/home/me/{name}")),
        }
    }

    #[test]
    fn entries_come_back_newest_first() {
        let store = HistoryStore::in_memory().unwrap();
        store.record(&sent_file("a.jpg")).unwrap();
        store.record(&sent_file("b.jpg")).unwrap();

        let labels: Vec<_> = store
            .list(10)
            .unwrap()
            .into_iter()
            .map(|e| e.item.label)
            .collect();
        assert_eq!(labels, ["b.jpg", "a.jpg"]);
    }

    #[test]
    fn a_received_file_remembers_where_it_was_moved() {
        let store = HistoryStore::in_memory().unwrap();
        let id = store
            .record(&Item {
                direction: Direction::Received,
                location: None,
                ..sent_file("scan.pdf")
            })
            .unwrap();
        store.set_location(id, "content://downloads/7").unwrap();

        let entry = store.list(1).unwrap().remove(0);
        assert_eq!(entry.item.direction, Direction::Received);
        assert_eq!(
            entry.item.location.as_deref(),
            Some("content://downloads/7")
        );
    }

    #[test]
    fn only_the_newest_entries_are_kept() {
        let store = HistoryStore::in_memory().unwrap();
        for n in 0..HISTORY_LIMIT + 5 {
            store.record(&sent_file(&format!("{n}.jpg"))).unwrap();
        }

        let entries = store.list(HISTORY_LIMIT + 5).unwrap();
        assert_eq!(entries.len(), HISTORY_LIMIT as usize);
        assert_eq!(entries[0].item.label, format!("{}.jpg", HISTORY_LIMIT + 4));
    }

    #[test]
    fn long_text_is_cut_short_and_clear_empties_everything() {
        let store = HistoryStore::in_memory().unwrap();
        store
            .record(&Item {
                kind: Kind::Text,
                label: "x".repeat(TEXT_LIMIT + 10),
                ..sent_file("")
            })
            .unwrap();
        assert_eq!(store.list(1).unwrap()[0].item.label.len(), TEXT_LIMIT);

        store.clear().unwrap();
        assert!(store.list(10).unwrap().is_empty());
    }
}
