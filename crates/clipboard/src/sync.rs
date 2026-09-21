// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::MAX_FRAME_CLIPBOARD_BYTES;
use protocol::v1::{ClipboardAck, ClipboardFormat, ClipboardUpdate};
use sha2::{Digest, Sha256};

use crate::error::ClipboardError;
use crate::wire::{read_msg, write_msg};

pub struct ClipboardSynchronizer {
    local_sequence: AtomicU64,
    last_remote_sequence: AtomicU64,
    last_remote_timestamp: AtomicU64,
    recent_hashes: Mutex<Vec<[u8; 32]>>,
}

impl ClipboardSynchronizer {
    pub fn new() -> Self {
        Self {
            local_sequence: AtomicU64::new(0),
            last_remote_sequence: AtomicU64::new(0),
            last_remote_timestamp: AtomicU64::new(0),
            recent_hashes: Mutex::new(Vec::with_capacity(16)),
        }
    }

    fn compute_hash(format: ClipboardFormat, payload: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update((format as i32).to_be_bytes());
        hasher.update(payload);
        hasher.finalize().into()
    }

    fn register_local_hash(&self, hash: [u8; 32]) {
        let mut lock = self.recent_hashes.lock().unwrap();
        if lock.len() >= 16 {
            lock.remove(0);
        }
        lock.push(hash);
    }

    fn is_recent_hash(&self, hash: &[u8; 32]) -> bool {
        let lock = self.recent_hashes.lock().unwrap();
        lock.contains(hash)
    }

    pub async fn send_update(
        &self,
        stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        format: ClipboardFormat,
        payload: Vec<u8>,
        query: &CapabilityQuery,
    ) -> Result<ClipboardAck, ClipboardError> {
        evaluate_capability(query)?;

        if payload.len() > MAX_FRAME_CLIPBOARD_BYTES {
            return Err(ClipboardError::PayloadTooLarge {
                size: payload.len(),
                limit: MAX_FRAME_CLIPBOARD_BYTES,
            });
        }

        let hash = Self::compute_hash(format, &payload);
        self.register_local_hash(hash);

        let seq = self.local_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let update = ClipboardUpdate {
            sequence_number: seq,
            timestamp: ts,
            format: format as i32,
            payload,
        };

        write_msg(stream, &update).await?;
        let ack: ClipboardAck = read_msg(recv_stream).await?;

        if !ack.applied {
            return Err(ClipboardError::Rejected(ack.error_message));
        }

        Ok(ack)
    }

    pub async fn receive_update<F>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        apply_fn: F,
    ) -> Result<ClipboardUpdate, ClipboardError>
    where
        F: FnOnce(ClipboardFormat, &[u8]) -> Result<(), String>,
    {
        evaluate_capability(query)?;

        let update: ClipboardUpdate = read_msg(recv_stream).await?;

        if update.payload.len() > MAX_FRAME_CLIPBOARD_BYTES {
            let _ = write_msg(
                send_stream,
                &ClipboardAck {
                    sequence_number: update.sequence_number,
                    applied: false,
                    error_message: "Payload exceeds maximum allowed frame bytes".to_string(),
                },
            )
            .await;
            return Err(ClipboardError::PayloadTooLarge {
                size: update.payload.len(),
                limit: MAX_FRAME_CLIPBOARD_BYTES,
            });
        }

        let format = match update.format {
            1 => ClipboardFormat::ClipboardFormatTextPlain,
            2 => ClipboardFormat::ClipboardFormatTextHtml,
            3 => ClipboardFormat::ClipboardFormatImagePng,
            _ => ClipboardFormat::ClipboardFormatUnspecified,
        };

        let hash = Self::compute_hash(format, &update.payload);

        if self.is_recent_hash(&hash) {
            write_msg(
                send_stream,
                &ClipboardAck {
                    sequence_number: update.sequence_number,
                    applied: false,
                    error_message: "Suppressed echo loop".to_string(),
                },
            )
            .await?;
            return Ok(update);
        }

        let last_seq = self.last_remote_sequence.load(Ordering::SeqCst);
        if update.sequence_number <= last_seq {
            let _ = write_msg(
                send_stream,
                &ClipboardAck {
                    sequence_number: update.sequence_number,
                    applied: false,
                    error_message: "Stale sequence number".to_string(),
                },
            )
            .await;
            return Err(ClipboardError::StaleSequence {
                received: update.sequence_number,
                current: last_seq,
            });
        }

        match apply_fn(format, &update.payload) {
            Ok(()) => {
                self.last_remote_sequence
                    .store(update.sequence_number, Ordering::SeqCst);
                self.last_remote_timestamp
                    .store(update.timestamp, Ordering::SeqCst);
                self.register_local_hash(hash);

                write_msg(
                    send_stream,
                    &ClipboardAck {
                        sequence_number: update.sequence_number,
                        applied: true,
                        error_message: String::new(),
                    },
                )
                .await?;

                Ok(update)
            }
            Err(e) => {
                write_msg(
                    send_stream,
                    &ClipboardAck {
                        sequence_number: update.sequence_number,
                        applied: false,
                        error_message: e.clone(),
                    },
                )
                .await?;
                Err(ClipboardError::Rejected(e))
            }
        }
    }
}

impl Default for ClipboardSynchronizer {
    fn default() -> Self {
        Self::new()
    }
}
