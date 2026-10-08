// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use protocol::v1::ClipboardFormat;
use rusqlite::{params, Connection};

use crate::error::ClipboardError;
use crate::guard::{evaluate_clipboard_formats, SensitiveGuardDecision};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardHistoryEntry {
    pub id: i64,
    pub timestamp_ms: u64,
    pub format: ClipboardFormat,
    pub content: String,
    pub is_pinned: bool,
    pub origin_device: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardSnippet {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub shortcut: Option<String>,
    pub created_at_ms: u64,
}

#[derive(Clone)]
pub struct ClipboardHistoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl ClipboardHistoryStore {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self, ClipboardError> {
        Self::new(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self, ClipboardError> {
        Self::new(Connection::open_in_memory()?)
    }

    fn new(conn: Connection) -> Result<Self, ClipboardError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ms  INTEGER NOT NULL,
                format        INTEGER NOT NULL,
                content       TEXT NOT NULL,
                is_pinned     INTEGER NOT NULL DEFAULT 0,
                origin_device TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_ts ON clipboard_history(timestamp_ms DESC);
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_pinned ON clipboard_history(is_pinned);

            CREATE TABLE IF NOT EXISTS clipboard_snippets (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                title         TEXT NOT NULL,
                content       TEXT NOT NULL,
                shortcut      TEXT,
                created_at_ms INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_clipboard_snippets_shortcut ON clipboard_snippets(shortcut);",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn record_clip<I, S>(
        &self,
        format: ClipboardFormat,
        content: &str,
        origin_device: &str,
        available_formats: Option<I>,
    ) -> Result<Option<i64>, ClipboardError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        if let Some(formats) = available_formats {
            if matches!(
                evaluate_clipboard_formats(formats),
                SensitiveGuardDecision::DropSensitive
            ) {
                return Ok(None);
            }
        }

        if content.is_empty() || format == ClipboardFormat::Unspecified {
            return Ok(None);
        }

        let format_id = format as i32;
        let ts = now_ms();
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, content, format, is_pinned FROM clipboard_history ORDER BY id DESC LIMIT 1;",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let last_id: i64 = row.get(0)?;
            let last_content: String = row.get(1)?;
            let last_format: i32 = row.get(2)?;
            let is_pinned: bool = row.get::<_, i64>(3)? != 0;

            if !is_pinned && last_format == format_id && last_content == content {
                conn.execute(
                    "UPDATE clipboard_history SET timestamp_ms = ?1, origin_device = ?2 WHERE id = ?3;",
                    params![ts as i64, origin_device, last_id],
                )?;
                return Ok(Some(last_id));
            }
        }

        conn.execute(
            "INSERT INTO clipboard_history (timestamp_ms, format, content, is_pinned, origin_device)
             VALUES (?1, ?2, ?3, 0, ?4);",
            params![ts as i64, format_id, content, origin_device],
        )?;
        Ok(Some(conn.last_insert_rowid()))
    }

    pub fn pin_clip(&self, id: i64, pinned: bool) -> Result<bool, ClipboardError> {
        let rows_affected = self.conn.lock().unwrap().execute(
            "UPDATE clipboard_history SET is_pinned = ?1 WHERE id = ?2;",
            params![if pinned { 1 } else { 0 }, id],
        )?;
        Ok(rows_affected > 0)
    }

    pub fn delete_clip(&self, id: i64) -> Result<bool, ClipboardError> {
        let rows_affected = self
            .conn
            .lock()
            .unwrap()
            .execute("DELETE FROM clipboard_history WHERE id = ?1;", params![id])?;
        Ok(rows_affected > 0)
    }

    pub fn clear_unpinned(&self) -> Result<usize, ClipboardError> {
        let rows_affected = self
            .conn
            .lock()
            .unwrap()
            .execute("DELETE FROM clipboard_history WHERE is_pinned = 0;", [])?;
        Ok(rows_affected)
    }

    pub fn cleanup_expired(
        &self,
        max_age_ms: u64,
        max_unpinned: u32,
    ) -> Result<usize, ClipboardError> {
        let now = now_ms();
        let cutoff = now.saturating_sub(max_age_ms);
        let conn = self.conn.lock().unwrap();

        let mut deleted = conn.execute(
            "DELETE FROM clipboard_history WHERE is_pinned = 0 AND timestamp_ms < ?1;",
            params![cutoff as i64],
        )?;

        let mut count_stmt =
            conn.prepare("SELECT COUNT(*) FROM clipboard_history WHERE is_pinned = 0;")?;
        let unpinned_count: u32 = count_stmt.query_row([], |row| row.get(0))?;

        if unpinned_count > max_unpinned {
            let excess = unpinned_count - max_unpinned;
            let trimmed = conn.execute(
                "DELETE FROM clipboard_history WHERE id IN (
                    SELECT id FROM clipboard_history WHERE is_pinned = 0 ORDER BY timestamp_ms ASC LIMIT ?1
                );",
                params![excess],
            )?;
            deleted += trimmed;
        }

        Ok(deleted)
    }

    pub fn list_clips(&self, limit: u32) -> Result<Vec<ClipboardHistoryEntry>, ClipboardError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp_ms, format, content, is_pinned, origin_device
             FROM clipboard_history ORDER BY is_pinned DESC, timestamp_ms DESC LIMIT ?1;",
        )?;
        let entries = stmt
            .query_map(params![limit], Self::map_entry_row)?
            .collect::<Result<_, _>>()?;
        Ok(entries)
    }

    pub fn search_clips(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<ClipboardHistoryEntry>, ClipboardError> {
        let pattern = format!("%{query}%");
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp_ms, format, content, is_pinned, origin_device
             FROM clipboard_history
             WHERE content LIKE ?1
             ORDER BY is_pinned DESC, timestamp_ms DESC
             LIMIT ?2;",
        )?;
        let entries = stmt
            .query_map(params![pattern, limit], Self::map_entry_row)?
            .collect::<Result<_, _>>()?;
        Ok(entries)
    }

    pub fn add_snippet(
        &self,
        title: &str,
        content: &str,
        shortcut: Option<&str>,
    ) -> Result<i64, ClipboardError> {
        let ts = now_ms();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO clipboard_snippets (title, content, shortcut, created_at_ms)
             VALUES (?1, ?2, ?3, ?4);",
            params![title, content, shortcut, ts as i64],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_snippets(&self) -> Result<Vec<ClipboardSnippet>, ClipboardError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, content, shortcut, created_at_ms
             FROM clipboard_snippets ORDER BY id ASC;",
        )?;
        let snippets = stmt
            .query_map([], |row| {
                Ok(ClipboardSnippet {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    shortcut: row.get(3)?,
                    created_at_ms: row.get::<_, i64>(4)? as u64,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(snippets)
    }

    pub fn find_snippet_by_shortcut(
        &self,
        shortcut: &str,
    ) -> Result<Option<ClipboardSnippet>, ClipboardError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, content, shortcut, created_at_ms
             FROM clipboard_snippets WHERE shortcut = ?1 LIMIT 1;",
        )?;
        let mut rows = stmt.query(params![shortcut])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ClipboardSnippet {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                shortcut: row.get(3)?,
                created_at_ms: row.get::<_, i64>(4)? as u64,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_snippet(&self, id: i64) -> Result<bool, ClipboardError> {
        let rows_affected = self
            .conn
            .lock()
            .unwrap()
            .execute("DELETE FROM clipboard_snippets WHERE id = ?1;", params![id])?;
        Ok(rows_affected > 0)
    }

    fn map_entry_row(row: &rusqlite::Row<'_>) -> Result<ClipboardHistoryEntry, rusqlite::Error> {
        let format_raw: i32 = row.get(2)?;
        let format = match format_raw {
            1 => ClipboardFormat::TextPlain,
            2 => ClipboardFormat::TextHtml,
            3 => ClipboardFormat::ImagePng,
            _ => ClipboardFormat::Unspecified,
        };
        Ok(ClipboardHistoryEntry {
            id: row.get(0)?,
            timestamp_ms: row.get::<_, i64>(1)? as u64,
            format,
            content: row.get(3)?,
            is_pinned: row.get::<_, i64>(4)? != 0,
            origin_device: row.get(5)?,
        })
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

    #[test]
    fn records_and_lists_clips() {
        let store = ClipboardHistoryStore::in_memory().unwrap();
        let id1 = store
            .record_clip::<[&str; 0], _>(ClipboardFormat::TextPlain, "first clip", "phone", None)
            .unwrap();
        assert!(id1.is_some());

        let id2 = store
            .record_clip::<[&str; 0], _>(ClipboardFormat::TextPlain, "second clip", "pc", None)
            .unwrap();
        assert!(id2.is_some());

        let clips = store.list_clips(10).unwrap();
        assert_eq!(clips.len(), 2);
        assert_eq!(clips[0].content, "second clip");
        assert_eq!(clips[1].content, "first clip");
    }

    #[test]
    fn sensitive_formats_are_excluded() {
        let store = ClipboardHistoryStore::in_memory().unwrap();
        let guard_formats = ["CF_UNICODETEXT", "Clipboard Viewer Ignore"];
        let result = store
            .record_clip(
                ClipboardFormat::TextPlain,
                "super_secret_password",
                "pc",
                Some(guard_formats.as_slice()),
            )
            .unwrap();
        assert_eq!(result, None);

        let clips = store.list_clips(10).unwrap();
        assert!(clips.is_empty());
    }

    #[test]
    fn consecutive_duplicate_updates_timestamp() {
        let store = ClipboardHistoryStore::in_memory().unwrap();
        let id1 = store
            .record_clip::<[&str; 0], _>(ClipboardFormat::TextPlain, "duplicate", "pc", None)
            .unwrap()
            .unwrap();
        let id2 = store
            .record_clip::<[&str; 0], _>(ClipboardFormat::TextPlain, "duplicate", "pc", None)
            .unwrap()
            .unwrap();
        assert_eq!(id1, id2);

        let clips = store.list_clips(10).unwrap();
        assert_eq!(clips.len(), 1);
    }

    #[test]
    fn pinning_preserves_across_clear_and_cleanup() {
        let store = ClipboardHistoryStore::in_memory().unwrap();
        let _id1 = store
            .record_clip::<[&str; 0], _>(ClipboardFormat::TextPlain, "temp clip", "pc", None)
            .unwrap()
            .unwrap();
        let id2 = store
            .record_clip::<[&str; 0], _>(ClipboardFormat::TextPlain, "pinned clip", "pc", None)
            .unwrap()
            .unwrap();

        assert!(store.pin_clip(id2, true).unwrap());

        let removed = store.clear_unpinned().unwrap();
        assert_eq!(removed, 1);

        let remaining = store.list_clips(10).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, id2);
        assert!(remaining[0].is_pinned);
        assert_eq!(remaining[0].content, "pinned clip");

        // Verify unpinning and deleting
        assert!(store.pin_clip(id2, false).unwrap());
        assert!(store.delete_clip(id2).unwrap());
        assert!(store.list_clips(10).unwrap().is_empty());
    }

    #[test]
    fn search_clips_filters_by_substring() {
        let store = ClipboardHistoryStore::in_memory().unwrap();
        store
            .record_clip::<[&str; 0], _>(
                ClipboardFormat::TextPlain,
                "http://example.com/login",
                "phone",
                None,
            )
            .unwrap();
        store
            .record_clip::<[&str; 0], _>(
                ClipboardFormat::TextPlain,
                "meeting at 3pm",
                "phone",
                None,
            )
            .unwrap();

        let matches = store.search_clips("example.com", 10).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].content, "http://example.com/login");

        let empty = store.search_clips("nonexistent", 10).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn snippets_lifecycle_and_shortcut_lookup() {
        let store = ClipboardHistoryStore::in_memory().unwrap();
        let snippet_id = store
            .add_snippet("Home Address", "123 Main St, Springfield", Some("/addr"))
            .unwrap();

        let list = store.list_snippets().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Home Address");

        let found = store.find_snippet_by_shortcut("/addr").unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.content, "123 Main St, Springfield");

        assert!(store.delete_snippet(snippet_id).unwrap());
        assert!(store.list_snippets().unwrap().is_empty());
    }
}
