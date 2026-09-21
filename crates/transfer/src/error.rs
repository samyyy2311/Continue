// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransferError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Transport error: {0}")]
    Transport(#[from] transport::TransportError),

    #[error("QUIC connection error: {0}")]
    QuinnConnection(#[from] quinn::ConnectionError),

    #[error("QUIC stream read error: {0}")]
    QuinnRead(#[from] quinn::ReadError),

    #[error("QUIC stream write error: {0}")]
    QuinnWrite(#[from] quinn::WriteError),

    #[error("Protocol error: {0}")]
    Protocol(#[from] protocol::ProtocolError),

    #[error("Invalid filename: {0}")]
    InvalidFilename(String),

    #[error("Transfer rejected by receiver: {0}")]
    Rejected(String),

    #[error("Receiver busy")]
    Busy,

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("File size mismatch: expected {expected} bytes, received {actual} bytes")]
    SizeMismatch { expected: u64, actual: u64 },

    #[error("Unexpected response from peer")]
    UnexpectedResponse,
}
