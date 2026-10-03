// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Files on their way in, so apps can show how far along each is and stop one part way, and
//! the folder they're saved to, which the user can change at any time.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use tokio::sync::Notify;

/// A file being received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomingFile {
    pub peer: String,
    pub transfer_id: String,
    pub file_name: String,
    pub received: u64,
    pub total: u64,
}

/// What apps hear as files come in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncomingEvent {
    /// A file started, or moved on by at least a percent.
    Progress(IncomingFile),
    /// A file is no longer coming in: it arrived, failed or was cancelled. An arrival is also
    /// reported through `on_file_received`.
    Ended { peer: String, transfer_id: String },
}

pub type IncomingListener = Arc<dyn Fn(IncomingEvent) + Send + Sync>;

struct Entry {
    file: IncomingFile,
    stop: Arc<Notify>,
}

/// Every file being received, across all sessions. Cheap to clone; clones share the list.
#[derive(Clone, Default)]
pub struct IncomingFiles {
    entries: Arc<Mutex<Vec<Entry>>>,
    listener: Option<IncomingListener>,
}

impl IncomingFiles {
    pub fn with_listener(listener: IncomingListener) -> Self {
        Self {
            entries: Arc::default(),
            listener: Some(listener),
        }
    }

    /// The files coming in right now, oldest first.
    pub fn list(&self) -> Vec<IncomingFile> {
        let entries = self.entries.lock().unwrap();
        entries.iter().map(|entry| entry.file.clone()).collect()
    }

    /// Stops a file part way. Returns false if it isn't coming in any more.
    pub fn cancel(&self, transfer_id: &str) -> bool {
        let entries = self.entries.lock().unwrap();
        let entry = entries.iter().find(|e| e.file.transfer_id == transfer_id);
        entry.map(|e| e.stop.notify_one()).is_some()
    }

    /// Notes progress on a file, adding it the first time. `stop` is what `cancel` signals.
    pub(crate) fn update(
        &self,
        peer: &str,
        request: &protocol::v1::FileTransferRequest,
        received: u64,
        stop: &Arc<Notify>,
    ) {
        let changed = {
            let mut entries = self.entries.lock().unwrap();
            match entries.iter_mut().find(|e| Arc::ptr_eq(&e.stop, stop)) {
                Some(entry) => {
                    let moved = percent(received, request.file_size)
                        != percent(entry.file.received, request.file_size);
                    entry.file.received = received;
                    moved.then(|| entry.file.clone())
                }
                None => {
                    let file = IncomingFile {
                        peer: peer.to_string(),
                        transfer_id: request.transfer_id.clone(),
                        file_name: request.file_name.clone(),
                        received,
                        total: request.file_size,
                    };
                    entries.push(Entry {
                        file: file.clone(),
                        stop: stop.clone(),
                    });
                    Some(file)
                }
            }
        };
        if let (Some(file), Some(listener)) = (changed, &self.listener) {
            listener(IncomingEvent::Progress(file));
        }
    }

    /// Forgets the file `stop` belongs to, if it got as far as being listed.
    pub(crate) fn end(&self, stop: &Arc<Notify>) {
        let ended = {
            let mut entries = self.entries.lock().unwrap();
            let index = entries.iter().position(|e| Arc::ptr_eq(&e.stop, stop));
            index.map(|i| entries.remove(i).file)
        };
        if let (Some(file), Some(listener)) = (ended, &self.listener) {
            listener(IncomingEvent::Ended {
                peer: file.peer,
                transfer_id: file.transfer_id,
            });
        }
    }
}

fn percent(received: u64, total: u64) -> u64 {
    if total == 0 {
        100
    } else {
        received.saturating_mul(100) / total
    }
}

/// Where received files are saved. Cheap to clone; clones share the folder, so changing it
/// applies to every session, including ones already connected.
#[derive(Clone)]
pub struct SaveFolder(Arc<RwLock<PathBuf>>);

impl SaveFolder {
    pub fn new(folder: impl Into<PathBuf>) -> Self {
        Self(Arc::new(RwLock::new(folder.into())))
    }

    pub fn get(&self) -> PathBuf {
        self.0.read().unwrap().clone()
    }

    pub fn set(&self, folder: impl Into<PathBuf>) {
        *self.0.write().unwrap() = folder.into();
    }
}
