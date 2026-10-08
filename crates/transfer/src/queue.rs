// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::TransferError;

pub const DEFAULT_MAX_ITEMS_PER_PEER: usize = 50;
pub const DEFAULT_MAX_TOTAL_ITEMS: usize = 500;

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn generate_item_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let rand_num: u64 = rng.gen();
    format!("{:016x}", rand_num)
}

/// Payload stored in the offline queue awaiting peer connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueuedPayload {
    /// File stored locally to send when peer is reachable.
    File {
        path: PathBuf,
        relative_name: String,
        mime_type: Option<String>,
    },
    /// Clipboard content to sync.
    Clipboard { content: Vec<u8>, mime_type: String },
    /// Handoff item (e.g. active tab URL).
    Handoff { uri: String, title: Option<String> },
}

/// Individual item held in the offline queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedTransfer {
    pub id: String,
    pub peer_fingerprint: String,
    pub payload: QueuedPayload,
    pub queued_at: u64,
}

#[derive(Debug, Default)]
struct QueueInner {
    queues: HashMap<String, VecDeque<QueuedTransfer>>,
    total_count: usize,
}

/// Thread-safe in-memory queue for staging transfers destined for currently unreachable peers.
#[derive(Debug, Clone)]
pub struct OfflineTransferQueue {
    inner: Arc<Mutex<QueueInner>>,
    max_items_per_peer: usize,
    max_total_items: usize,
}

impl Default for OfflineTransferQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl OfflineTransferQueue {
    pub fn new() -> Self {
        Self::with_capacities(DEFAULT_MAX_ITEMS_PER_PEER, DEFAULT_MAX_TOTAL_ITEMS)
    }

    pub fn with_capacities(max_items_per_peer: usize, max_total_items: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(QueueInner::default())),
            max_items_per_peer,
            max_total_items,
        }
    }

    /// Enqueues a payload for the given peer fingerprint.
    pub fn enqueue(
        &self,
        peer_fingerprint: &str,
        payload: QueuedPayload,
    ) -> Result<String, TransferError> {
        let mut inner = self.inner.lock().unwrap();

        if inner.total_count >= self.max_total_items {
            return Err(TransferError::QueueFull(format!(
                "Total offline queue capacity ({}) exceeded",
                self.max_total_items
            )));
        }

        let peer_queue = inner
            .queues
            .entry(peer_fingerprint.to_string())
            .or_default();

        if peer_queue.len() >= self.max_items_per_peer {
            return Err(TransferError::QueueFull(format!(
                "Peer offline queue capacity ({}) exceeded for {}",
                self.max_items_per_peer, peer_fingerprint
            )));
        }

        let id = generate_item_id();
        let item = QueuedTransfer {
            id: id.clone(),
            peer_fingerprint: peer_fingerprint.to_string(),
            payload,
            queued_at: current_timestamp(),
        };

        peer_queue.push_back(item);
        inner.total_count += 1;

        Ok(id)
    }

    /// Drains all queued transfers for the specified peer in FIFO order.
    pub fn drain_for_peer(&self, peer_fingerprint: &str) -> Vec<QueuedTransfer> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(queue) = inner.queues.remove(peer_fingerprint) {
            inner.total_count = inner.total_count.saturating_sub(queue.len());
            queue.into()
        } else {
            Vec::new()
        }
    }

    /// Returns a snapshot of queued transfers for the peer without removing them.
    pub fn peek_for_peer(&self, peer_fingerprint: &str) -> Vec<QueuedTransfer> {
        let inner = self.inner.lock().unwrap();
        inner
            .queues
            .get(peer_fingerprint)
            .map(|q| q.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Cancels and removes a specific queued item by ID.
    pub fn remove_item(&self, peer_fingerprint: &str, item_id: &str) -> bool {
        let mut inner = self.inner.lock().unwrap();
        let (removed, is_empty) = if let Some(queue) = inner.queues.get_mut(peer_fingerprint) {
            let removed = if let Some(idx) = queue.iter().position(|item| item.id == item_id) {
                queue.remove(idx);
                true
            } else {
                false
            };
            (removed, queue.is_empty())
        } else {
            (false, false)
        };

        if removed {
            inner.total_count = inner.total_count.saturating_sub(1);
        }
        if is_empty {
            inner.queues.remove(peer_fingerprint);
        }

        removed
    }

    /// Clears all queued items for a peer.
    pub fn clear_peer(&self, peer_fingerprint: &str) -> usize {
        let mut inner = self.inner.lock().unwrap();
        if let Some(queue) = inner.queues.remove(peer_fingerprint) {
            let count = queue.len();
            inner.total_count = inner.total_count.saturating_sub(count);
            count
        } else {
            0
        }
    }

    /// Number of items pending for a specific peer.
    pub fn pending_count_for_peer(&self, peer_fingerprint: &str) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.queues.get(peer_fingerprint).map_or(0, |q| q.len())
    }

    /// Total items pending across all peers.
    pub fn total_pending_count(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        inner.total_count
    }

    /// List of peer fingerprints that currently have items pending in the queue.
    pub fn peers_with_pending(&self) -> Vec<String> {
        let inner = self.inner.lock().unwrap();
        inner.queues.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_and_drain_fifo_order() {
        let queue = OfflineTransferQueue::new();
        let peer = "peer-alpha";

        let id1 = queue
            .enqueue(
                peer,
                QueuedPayload::Handoff {
                    uri: "https://example.com/item1".to_string(),
                    title: Some("Item 1".to_string()),
                },
            )
            .unwrap();

        let id2 = queue
            .enqueue(
                peer,
                QueuedPayload::Clipboard {
                    content: b"clipboard text".to_vec(),
                    mime_type: "text/plain".to_string(),
                },
            )
            .unwrap();

        assert_eq!(queue.pending_count_for_peer(peer), 2);
        assert_eq!(queue.total_pending_count(), 2);

        let drained = queue.drain_for_peer(peer);
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].id, id1);
        assert_eq!(drained[1].id, id2);

        assert_eq!(queue.pending_count_for_peer(peer), 0);
        assert_eq!(queue.total_pending_count(), 0);
    }

    #[test]
    fn peer_isolation() {
        let queue = OfflineTransferQueue::new();
        let peer1 = "peer-1";
        let peer2 = "peer-2";

        queue
            .enqueue(
                peer1,
                QueuedPayload::Handoff {
                    uri: "https://continue.dev".to_string(),
                    title: None,
                },
            )
            .unwrap();

        assert_eq!(queue.pending_count_for_peer(peer1), 1);
        assert_eq!(queue.pending_count_for_peer(peer2), 0);

        let drained_p2 = queue.drain_for_peer(peer2);
        assert!(drained_p2.is_empty());

        let drained_p1 = queue.drain_for_peer(peer1);
        assert_eq!(drained_p1.len(), 1);
    }

    #[test]
    fn peek_does_not_consume() {
        let queue = OfflineTransferQueue::new();
        let peer = "peer-beta";

        queue
            .enqueue(
                peer,
                QueuedPayload::Clipboard {
                    content: b"snippet".to_vec(),
                    mime_type: "text/plain".to_string(),
                },
            )
            .unwrap();

        let peeked = queue.peek_for_peer(peer);
        assert_eq!(peeked.len(), 1);
        assert_eq!(queue.pending_count_for_peer(peer), 1);

        let drained = queue.drain_for_peer(peer);
        assert_eq!(drained.len(), 1);
    }

    #[test]
    fn remove_item_by_id() {
        let queue = OfflineTransferQueue::new();
        let peer = "peer-gamma";

        let id1 = queue
            .enqueue(
                peer,
                QueuedPayload::File {
                    path: PathBuf::from("photo.png"),
                    relative_name: "photo.png".to_string(),
                    mime_type: Some("image/png".to_string()),
                },
            )
            .unwrap();

        let id2 = queue
            .enqueue(
                peer,
                QueuedPayload::Handoff {
                    uri: "https://example.com".to_string(),
                    title: None,
                },
            )
            .unwrap();

        assert_eq!(queue.total_pending_count(), 2);
        assert!(queue.remove_item(peer, &id1));
        assert_eq!(queue.total_pending_count(), 1);
        assert_eq!(queue.pending_count_for_peer(peer), 1);

        let remaining = queue.drain_for_peer(peer);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, id2);
    }

    #[test]
    fn capacity_limits_enforced() {
        let queue = OfflineTransferQueue::with_capacities(2, 3);
        let peer_a = "peer-a";
        let peer_b = "peer-b";

        assert!(queue
            .enqueue(
                peer_a,
                QueuedPayload::Clipboard {
                    content: vec![1],
                    mime_type: "text/plain".to_string(),
                }
            )
            .is_ok());
        assert!(queue
            .enqueue(
                peer_a,
                QueuedPayload::Clipboard {
                    content: vec![2],
                    mime_type: "text/plain".to_string(),
                }
            )
            .is_ok());

        // Peer limit reached
        assert!(matches!(
            queue.enqueue(
                peer_a,
                QueuedPayload::Clipboard {
                    content: vec![3],
                    mime_type: "text/plain".to_string(),
                }
            ),
            Err(TransferError::QueueFull(_))
        ));

        // Another peer can enqueue up to total capacity
        assert!(queue
            .enqueue(
                peer_b,
                QueuedPayload::Clipboard {
                    content: vec![4],
                    mime_type: "text/plain".to_string(),
                }
            )
            .is_ok());

        // Total limit reached (3 items)
        assert!(matches!(
            queue.enqueue(
                peer_b,
                QueuedPayload::Clipboard {
                    content: vec![5],
                    mime_type: "text/plain".to_string(),
                }
            ),
            Err(TransferError::QueueFull(_))
        ));
    }

    #[test]
    fn clear_peer_and_peers_with_pending() {
        let queue = OfflineTransferQueue::new();
        let peer1 = "peer-1";
        let peer2 = "peer-2";

        queue
            .enqueue(
                peer1,
                QueuedPayload::Clipboard {
                    content: vec![1],
                    mime_type: "text/plain".to_string(),
                },
            )
            .unwrap();
        queue
            .enqueue(
                peer2,
                QueuedPayload::Clipboard {
                    content: vec![2],
                    mime_type: "text/plain".to_string(),
                },
            )
            .unwrap();

        let mut peers = queue.peers_with_pending();
        peers.sort();
        assert_eq!(peers, vec!["peer-1".to_string(), "peer-2".to_string()]);

        assert_eq!(queue.clear_peer(peer1), 1);
        assert_eq!(queue.peers_with_pending(), vec!["peer-2".to_string()]);
        assert_eq!(queue.total_pending_count(), 1);
    }
}
