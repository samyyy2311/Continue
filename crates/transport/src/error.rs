// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("QUIC connection error: {0}")]
    QuinnConnection(#[from] quinn::ConnectionError),

    #[error("QUIC connect error: {0}")]
    QuinnConnect(#[from] quinn::ConnectError),

    #[error("QUIC stream write error: {0}")]
    StreamWrite(#[from] quinn::WriteError),

    #[error("QUIC stream read error: {0}")]
    StreamRead(#[from] quinn::ReadError),

    #[error("QUIC stream read error: {0}")]
    StreamReadExact(#[from] quinn::ReadExactError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TLS error: {0}")]
    Tls(#[from] rustls::Error),

    #[error("Certificate generation error: {0}")]
    Rcgen(#[from] rcgen::Error),

    #[error("Failed to extract SubjectPublicKeyInfo from certificate")]
    SpkiExtractionFailed,

    #[error("Peer transport SPKI hash mismatch: expected {expected:?}, got {actual:?}")]
    SpkiMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },

    #[error("Frame error: {0}")]
    Frame(#[from] protocol::FrameError),

    #[error("Timed out connecting to {0}")]
    ConnectTimeout(std::net::SocketAddr),

    #[error("Connection closed by peer")]
    ConnectionClosed,

    #[error("Transport handshake failed: {0}")]
    HandshakeFailed(String),
}
