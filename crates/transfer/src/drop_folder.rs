// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use crate::error::TransferError;
use crate::queue::{OfflineTransferQueue, QueuedPayload, QueuedTransfer};

#[derive(Debug, Clone)]
pub struct DropFolderConfig {
    pub watch_dir: PathBuf,
    pub target_peer: String,
    pub debounce_duration: Duration,
    pub delete_after_enqueue: bool,
}

impl DropFolderConfig {
    pub fn new(watch_dir: PathBuf, target_peer: impl Into<String>) -> Self {
        Self {
            watch_dir,
            target_peer: target_peer.into(),
            debounce_duration: Duration::from_millis(500),
            delete_after_enqueue: false,
        }
    }
}

pub fn is_ignorable_file(file_name: &str) -> bool {
    if file_name.starts_with('.') || file_name.starts_with("~$") {
        return true;
    }

    let lower = file_name.to_ascii_lowercase();
    const IGNORED_EXTENSIONS: &[&str] = &[
        ".tmp",
        ".crdownload",
        ".part",
        ".partial",
        ".download",
        ".swp",
        ".bak",
    ];

    IGNORED_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

pub fn guess_mime_type(file_name: &str) -> Option<String> {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        Some("image/jpeg".to_string())
    } else if lower.ends_with(".png") {
        Some("image/png".to_string())
    } else if lower.ends_with(".gif") {
        Some("image/gif".to_string())
    } else if lower.ends_with(".webp") {
        Some("image/webp".to_string())
    } else if lower.ends_with(".pdf") {
        Some("application/pdf".to_string())
    } else if lower.ends_with(".txt") {
        Some("text/plain".to_string())
    } else if lower.ends_with(".mp4") {
        Some("video/mp4".to_string())
    } else if lower.ends_with(".zip") {
        Some("application/zip".to_string())
    } else {
        None
    }
}

#[derive(Debug, Clone)]
struct TrackedFile {
    last_size: u64,
    last_modified: SystemTime,
    last_change: Instant,
    enqueued: bool,
}

pub struct DropFolderWatcher {
    config: DropFolderConfig,
    queue: Arc<OfflineTransferQueue>,
    tracked: Mutex<HashMap<PathBuf, TrackedFile>>,
}

impl DropFolderWatcher {
    pub fn new(config: DropFolderConfig, queue: Arc<OfflineTransferQueue>) -> Self {
        Self {
            config,
            queue,
            tracked: Mutex::new(HashMap::new()),
        }
    }

    pub fn scan_directory(&self) -> Result<Vec<QueuedTransfer>, TransferError> {
        let watch_dir = &self.config.watch_dir;
        if !watch_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(watch_dir)?;
        let mut now_seen_paths = Vec::new();
        let mut newly_enqueued = Vec::new();
        let now = Instant::now();

        let mut tracked = self.tracked.lock().unwrap();

        for entry_res in entries {
            let entry = entry_res?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            if is_ignorable_file(&file_name) {
                continue;
            }

            let metadata = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size = metadata.len();
            let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);

            now_seen_paths.push(path.clone());

            let entry_state = tracked.entry(path.clone()).or_insert_with(|| TrackedFile {
                last_size: size,
                last_modified: modified,
                last_change: now,
                enqueued: false,
            });

            if entry_state.last_size != size || entry_state.last_modified != modified {
                entry_state.last_size = size;
                entry_state.last_modified = modified;
                entry_state.last_change = now;
                continue;
            }

            if !entry_state.enqueued
                && now.duration_since(entry_state.last_change) >= self.config.debounce_duration
            {
                let mime_type = guess_mime_type(&file_name);
                let payload = QueuedPayload::File {
                    path: path.clone(),
                    relative_name: file_name,
                    mime_type,
                };

                let item_id = self.queue.enqueue(&self.config.target_peer, payload)?;
                entry_state.enqueued = true;
                newly_enqueued.push(QueuedTransfer {
                    id: item_id,
                    peer_fingerprint: self.config.target_peer.clone(),
                    payload: QueuedPayload::File {
                        path: path.clone(),
                        relative_name: path.file_name().unwrap().to_string_lossy().into_owned(),
                        mime_type: guess_mime_type(&path.file_name().unwrap().to_string_lossy()),
                    },
                    queued_at: SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                });

                if self.config.delete_after_enqueue {
                    let _ = fs::remove_file(&path);
                }
            }
        }

        tracked.retain(|p, _| now_seen_paths.contains(p));

        Ok(newly_enqueued)
    }

    pub async fn run_polling(
        self: Arc<Self>,
        poll_interval: Duration,
        mut stop_rx: tokio::sync::watch::Receiver<bool>,
    ) {
        let mut interval = tokio::time::interval(poll_interval);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let _ = self.scan_directory();
                }
                changed = stop_rx.changed() => {
                    if changed.is_err() || *stop_rx.borrow() {
                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "continue_test_{}_{}",
                name,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let _ = fs::create_dir_all(&path);
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn ignorable_files_filtered() {
        assert!(is_ignorable_file(".hidden"));
        assert!(is_ignorable_file(".DS_Store"));
        assert!(is_ignorable_file("~$doc.docx"));
        assert!(is_ignorable_file("large.iso.crdownload"));
        assert!(is_ignorable_file("download.part"));
        assert!(is_ignorable_file("temp_file.tmp"));
        assert!(!is_ignorable_file("vacation.jpg"));
        assert!(!is_ignorable_file("document.pdf"));
    }

    #[test]
    fn debounces_and_enqueues_stable_file() {
        let dir = TestDir::new("drop_debounce");
        let queue = Arc::new(OfflineTransferQueue::with_capacities(10, 100));
        let mut config = DropFolderConfig::new(dir.path.clone(), "pixel_phone");
        config.debounce_duration = Duration::from_millis(100);

        let watcher = DropFolderWatcher::new(config, queue.clone());

        let file_path = dir.path.join("report.pdf");
        fs::write(&file_path, b"Test content for file transfer").unwrap();

        // Immediate scan before debounce elapsed should not enqueue yet
        let enqueued_first = watcher.scan_directory().unwrap();
        assert!(enqueued_first.is_empty());
        assert_eq!(queue.peek_for_peer("pixel_phone").len(), 0);

        // Wait for debounce period to pass
        std::thread::sleep(Duration::from_millis(150));

        let enqueued_second = watcher.scan_directory().unwrap();
        assert_eq!(enqueued_second.len(), 1);
        assert_eq!(queue.peek_for_peer("pixel_phone").len(), 1);

        // Subsequent scan does not duplicate enqueue
        let enqueued_third = watcher.scan_directory().unwrap();
        assert!(enqueued_third.is_empty());
        assert_eq!(queue.peek_for_peer("pixel_phone").len(), 1);
    }
}
