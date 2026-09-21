// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session is closed")]
    Closed,

    #[error("Session keepalive timed out (no Pong received)")]
    KeepaliveTimeout,

    #[error("Max reconnect attempts ({0}) exceeded")]
    MaxReconnectAttemptsExceeded(u32),

    #[error("Protocol error: {0}")]
    Protocol(#[from] protocol::ProtocolError),

    #[error("Transport error: {0}")]
    Transport(#[from] transport::TransportError),

    #[error("Unexpected session envelope payload")]
    UnexpectedMessage,
}
