// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, Notify};
use tracing::{debug, warn};

use limits::KEEPALIVE_INTERVAL_SECS;
use protocol::v1::{
    session_envelope::Body, Disconnect, DisconnectReason, Ping, Pong, SessionEnvelope,
};
use protocol::CapabilityId;

use crate::error::SessionError;
use crate::keepalive::KeepaliveTracker;
use crate::session::Session;

/// Header written at the start of any multiplexed QUIC bidirectional stream.
pub async fn open_capability_stream(
    conn: &quinn::Connection,
    capability: CapabilityId,
) -> Result<(quinn::SendStream, quinn::RecvStream), transport::TransportError> {
    let (mut send, recv) = conn.open_bi().await.map_err(transport::TransportError::from)?;
    send.write_all(&capability.raw().to_be_bytes())
        .await
        .map_err(transport::TransportError::from)?;
    Ok((send, recv))
}

/// Read the 4-byte capability identifier from the beginning of an incoming stream.
pub async fn read_capability_stream_header(
    recv: &mut quinn::RecvStream,
) -> Result<CapabilityId, transport::TransportError> {
    let mut buf = [0u8; 4];
    recv.read_exact(&mut buf)
        .await
        .map_err(transport::TransportError::from)?;
    Ok(CapabilityId(u32::from_be_bytes(buf)))
}

/// An incoming stream dispatched by the multiplexer router.
pub struct IncomingCapabilityStream {
    pub capability: CapabilityId,
    pub send_stream: quinn::SendStream,
    pub recv_stream: quinn::RecvStream,
}

/// Manages stream dispatching and control loop over an authenticated QUIC connection.
pub struct SessionMultiplexer {
    peer_fingerprint: String,
    connection: quinn::Connection,
    keepalive: Arc<Mutex<KeepaliveTracker>>,
    shutdown_notify: Arc<Notify>,
}

impl SessionMultiplexer {
    pub fn new(peer_fingerprint: String, connection: quinn::Connection) -> Self {
        Self {
            peer_fingerprint,
            connection,
            keepalive: Arc::new(Mutex::new(KeepaliveTracker::new())),
            shutdown_notify: Arc::new(Notify::new()),
        }
    }

    pub fn peer_fingerprint(&self) -> &str {
        &self.peer_fingerprint
    }

    pub fn connection(&self) -> &quinn::Connection {
        &self.connection
    }

    /// Open a dedicated stream for a specific capability.
    pub async fn open_stream(
        &self,
        capability: CapabilityId,
    ) -> Result<(quinn::SendStream, quinn::RecvStream), transport::TransportError> {
        open_capability_stream(&self.connection, capability).await
    }

    /// Run the background stream router. Incoming streams are tagged and sent to the returned channel.
    pub fn spawn_router(
        &self,
        stream_buffer_size: usize,
    ) -> mpsc::Receiver<IncomingCapabilityStream> {
        let (tx, rx) = mpsc::channel(stream_buffer_size);
        let conn = self.connection.clone();
        let shutdown = self.shutdown_notify.clone();
        let keepalive = self.keepalive.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    incoming = conn.accept_bi() => {
                        match incoming {
                            Ok((send, mut recv)) => {
                                match read_capability_stream_header(&mut recv).await {
                                    Ok(CapabilityId::CONTROL) => {
                                        let ka = keepalive.clone();
                                        tokio::spawn(Self::handle_control_stream(send, recv, ka));
                                    }
                                    Ok(cap) => {
                                        let item = IncomingCapabilityStream {
                                            capability: cap,
                                            send_stream: send,
                                            recv_stream: recv,
                                        };
                                        if tx.send(item).await.is_err() {
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        debug!("Failed to read capability stream header: {e}");
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        });

        rx
    }

    /// Run the periodic keepalive Ping loop on this session.
    pub fn spawn_keepalive_sender(&self) {
        let conn = self.connection.clone();
        let keepalive = self.keepalive.clone();
        let shutdown = self.shutdown_notify.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(KEEPALIVE_INTERVAL_SECS));
            loop {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = interval.tick() => {
                        let should_ping = {
                            let tracker = keepalive.lock().await;
                            if tracker.is_timed_out() {
                                warn!("Keepalive timed out for peer session");
                                conn.close(0u32.into(), b"keepalive timeout");
                                break;
                            }
                            tracker.should_ping()
                        };

                        if should_ping {
                            let ping_env = {
                                let mut tracker = keepalive.lock().await;
                                tracker.next_ping()
                            };

                            if let Ok((mut send, mut recv)) = open_capability_stream(&conn, CapabilityId::CONTROL).await {
                                if Session::send_envelope(&mut send, &ping_env).await.is_ok() {
                                    if let Ok(resp_env) = Session::read_envelope(&mut recv).await {
                                        if let Some(Body::Pong(Pong { seq })) = resp_env.body {
                                            let mut tracker = keepalive.lock().await;
                                            tracker.record_pong(seq);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// Internal handler for incoming control streams (answering Pings and Disconnects).
    async fn handle_control_stream(
        mut send: quinn::SendStream,
        mut recv: quinn::RecvStream,
        keepalive: Arc<Mutex<KeepaliveTracker>>,
    ) {
        if let Ok(env) = Session::read_envelope(&mut recv).await {
            match env.body {
                Some(Body::Ping(Ping { seq })) => {
                    let pong = KeepaliveTracker::make_pong(seq);
                    let _ = Session::send_envelope(&mut send, &pong).await;
                    let mut tracker = keepalive.lock().await;
                    tracker.record_received();
                }
                Some(Body::Disconnect(Disconnect { reason, message })) => {
                    debug!("Peer sent disconnect: reason={reason:?}, message={message}");
                }
                _ => {}
            }
        }
    }

    /// Disconnect session and release resources.
    pub async fn disconnect(&self, reason: DisconnectReason, message: String) {
        self.shutdown_notify.notify_waiters();

        if let Ok((mut send, _)) = open_capability_stream(&self.connection, CapabilityId::CONTROL).await {
            let env = SessionEnvelope {
                body: Some(Body::Disconnect(Disconnect {
                    reason: reason as i32,
                    message: message.clone(),
                })),
            };
            let _ = Session::send_envelope(&mut send, &env).await;
        }

        self.connection.close(0u32.into(), message.as_bytes());
    }
}
