// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

/// State of a peer session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// No connection active.
    Disconnected,
    /// Handshake / connection in progress.
    Connecting,
    /// Actively connected and authenticated.
    Connected,
    /// Connection dropped, attempting backoff reconnection.
    Reconnecting,
    /// Session explicitly terminated by user or unrecoverable error.
    Closed,
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Disconnected"),
            Self::Connecting => write!(f, "Connecting"),
            Self::Connected => write!(f, "Connected"),
            Self::Reconnecting => write!(f, "Reconnecting"),
            Self::Closed => write!(f, "Closed"),
        }
    }
}

impl SessionState {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Connected)
    }

    pub fn can_reconnect(&self) -> bool {
        !matches!(self, Self::Closed)
    }
}
