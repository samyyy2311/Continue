// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use bytes::BytesMut;

use limits::MAX_FRAME_SESSION_BYTES;
use protocol::v1::SessionEnvelope;
use transport::AuthenticatedPeerSession;

use crate::backoff::ReconnectPolicy;
use crate::error::SessionError;
use crate::keepalive::KeepaliveTracker;
use crate::state::SessionState;

/// An active or reconnecting session with a trusted peer.
pub struct Session {
    pub peer_fingerprint: String,
    pub peer_transport_spki_hash: [u8; 32],
    pub known_addresses: Vec<SocketAddr>,
    pub state: SessionState,
    pub active_session: Option<AuthenticatedPeerSession>,
    pub keepalive: KeepaliveTracker,
    pub reconnect_policy: ReconnectPolicy,
}

impl Session {
    pub fn new(
        peer_fingerprint: String,
        peer_transport_spki_hash: [u8; 32],
        known_addresses: Vec<SocketAddr>,
    ) -> Self {
        Self {
            peer_fingerprint,
            peer_transport_spki_hash,
            known_addresses,
            state: SessionState::Disconnected,
            active_session: None,
            keepalive: KeepaliveTracker::new(),
            reconnect_policy: ReconnectPolicy::new(),
        }
    }

    /// Attach an authenticated QUIC connection to this session.
    pub fn attach_connection(&mut self, session: AuthenticatedPeerSession) {
        self.active_session = Some(session);
        self.state = SessionState::Connected;
        self.keepalive = KeepaliveTracker::new();
        self.reconnect_policy.reset();
    }

    /// Mark the connection as dropped and transition to Reconnecting.
    pub fn mark_disconnected(&mut self) -> Option<std::time::Duration> {
        self.active_session = None;
        if self.state.can_reconnect() {
            self.state = SessionState::Reconnecting;
            self.reconnect_policy.next_delay()
        } else {
            None
        }
    }

    /// Explicitly close the session.
    pub fn close(&mut self) {
        self.state = SessionState::Closed;
        self.active_session = None;
    }

    /// Send a control envelope on the session's stream.
    pub async fn send_envelope(
        send_stream: &mut quinn::SendStream,
        envelope: &SessionEnvelope,
    ) -> Result<(), SessionError> {
        let frame = protocol::encode_frame(envelope, MAX_FRAME_SESSION_BYTES)?;
        send_stream
            .write_all(&frame)
            .await
            .map_err(transport::TransportError::from)?;
        Ok(())
    }

    /// Read a control envelope from the session's stream.
    pub async fn read_envelope(
        recv_stream: &mut quinn::RecvStream,
    ) -> Result<SessionEnvelope, SessionError> {
        let mut buf = BytesMut::with_capacity(1024);
        let mut chunk = [0u8; 1024];
        loop {
            if let Some(msg) = protocol::decode_frame_from_buf::<SessionEnvelope>(&mut buf, MAX_FRAME_SESSION_BYTES)? {
                return Ok(msg);
            }
            match recv_stream.read(&mut chunk).await.map_err(transport::TransportError::from)? {
                Some(n) if n > 0 => {
                    buf.extend_from_slice(&chunk[..n]);
                }
                _ => return Err(transport::TransportError::ConnectionClosed.into()),
            }
        }
    }
}
