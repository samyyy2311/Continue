// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

/// Network path used for a peer connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionPath {
    /// Local LAN direct connection.
    LocalLan,
    /// Direct remote connection (e.g. over WAN or VPN).
    DirectRemote,
    /// Relayed via end-to-end encrypted relay.
    Relayed,
}

impl fmt::Display for ConnectionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalLan => write!(f, "local_lan"),
            Self::DirectRemote => write!(f, "direct_remote"),
            Self::Relayed => write!(f, "relayed"),
        }
    }
}

/// An authenticated, active transport session with a peer.
pub struct AuthenticatedPeerSession {
    /// The underlying QUIC connection.
    pub connection: quinn::Connection,
    /// Verified transport certificate SPKI hash of the remote peer.
    pub peer_transport_spki_hash: [u8; 32],
    /// Physical/network connection path type.
    pub path: ConnectionPath,
}

impl AuthenticatedPeerSession {
    pub fn new(
        connection: quinn::Connection,
        peer_transport_spki_hash: [u8; 32],
        path: ConnectionPath,
    ) -> Self {
        Self {
            connection,
            peer_transport_spki_hash,
            path,
        }
    }

    /// Open a bidirectional stream on the QUIC connection.
    pub async fn open_bi(&self) -> Result<(quinn::SendStream, quinn::RecvStream), quinn::ConnectionError> {
        self.connection.open_bi().await
    }

    /// Accept an incoming bidirectional stream on the QUIC connection.
    pub async fn accept_bi(&self) -> Result<(quinn::SendStream, quinn::RecvStream), quinn::ConnectionError> {
        self.connection.accept_bi().await
    }
}
