// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("Frame error: {0}")]
    Frame(#[from] protocol::FrameError),

    #[error("Transport error: {0}")]
    Transport(#[from] transport::TransportError),

    #[error("Capability error: {0}")]
    Capability(#[from] capabilities::CapabilityError),

    #[error("Clipboard payload too large: {size} bytes (limit: {limit})")]
    PayloadTooLarge { size: usize, limit: usize },

    #[error("Stale sequence number: received {received}, current {current}")]
    StaleSequence { received: u64, current: u64 },

    #[error("Rejected by peer: {0}")]
    Rejected(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
