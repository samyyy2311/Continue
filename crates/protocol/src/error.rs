// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("Frame length {size} exceeds maximum allowed limit {limit}")]
    FrameTooLarge { size: usize, limit: usize },

    #[error("Incomplete frame: needed {needed} bytes, but only {available} available")]
    Incomplete { needed: usize, available: usize },

    #[error("Failed to decode protobuf message: {0}")]
    Decode(#[from] prost::DecodeError),

    #[error("Failed to encode protobuf message: {0}")]
    Encode(#[from] prost::EncodeError),
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Frame error: {0}")]
    Frame(#[from] FrameError),

    #[error("Incompatible protocol version {peer_version} (supported range: {min_supported}..={max_supported})")]
    IncompatibleVersion {
        peer_version: u32,
        min_supported: u32,
        max_supported: u32,
    },
}
