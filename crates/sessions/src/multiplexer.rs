// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, Notify};
use tracing::{debug, warn};

use clipboard::ClipboardSynchronizer;
use limits::KEEPALIVE_INTERVAL_SECS;
use protocol::v1::{
    session_envelope::Body, DeviceInfo, DeviceLook, DeviceStatus, Disconnect, DisconnectReason,
    Ping, Pong, SessionEnvelope,
};
use protocol::CapabilityId;

use crate::keepalive::KeepaliveTracker;
use crate::session::Session;

/// Header written at the start of any multiplexed QUIC bidirectional stream.
pub async fn open_capability_stream(
    conn: &quinn::Connection,
    capability: CapabilityId,
) -> Result<(quinn::SendStream, quinn::RecvStream), transport::TransportError> {
    let (mut send, recv) = conn
        .open_bi()
        .await
        .map_err(transport::TransportError::from)?;
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

/// Something the peer said about itself on the control stream.
pub enum PeerUpdate {
    Info(DeviceInfo),
    Status(DeviceStatus),
    Look(DeviceLook),
}

pub type OnPeerUpdate = Arc<dyn Fn(PeerUpdate) + Send + Sync>;

/// QUIC application close code carrying a protocol `DisconnectReason`.
pub fn close_code(reason: DisconnectReason) -> quinn::VarInt {
    quinn::VarInt::from_u32(reason as u32)
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
    peer_disconnected: Arc<AtomicBool>,
    clipboard: Arc<ClipboardSynchronizer>,
}

impl SessionMultiplexer {
    pub fn new(peer_fingerprint: String, connection: quinn::Connection) -> Self {
        Self {
            peer_fingerprint,
            connection,
            keepalive: Arc::new(Mutex::new(KeepaliveTracker::new())),
            shutdown_notify: Arc::new(Notify::new()),
            peer_disconnected: Arc::new(AtomicBool::new(false)),
            clipboard: Arc::new(ClipboardSynchronizer::new()),
        }
    }

    pub fn peer_fingerprint(&self) -> &str {
        &self.peer_fingerprint
    }

    pub fn connection(&self) -> &quinn::Connection {
        &self.connection
    }

    /// Clipboard ordering and echo state, shared by both directions of this connection.
    /// Sequence numbers only have meaning within one connection, so a reconnect starts fresh.
    pub fn clipboard(&self) -> &Arc<ClipboardSynchronizer> {
        &self.clipboard
    }

    /// Whether the connection ended because the peer chose to leave, as opposed to a failure.
    /// Only a normal disconnect counts; errors and timeouts are worth reconnecting after.
    pub fn ended_by_peer(&self, error: &quinn::ConnectionError) -> bool {
        self.peer_disconnected.load(Ordering::Acquire)
            || matches!(
                error,
                quinn::ConnectionError::ApplicationClosed(close)
                    if close.error_code == close_code(DisconnectReason::Normal)
            )
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
        self.spawn_router_with(stream_buffer_size, None)
    }

    /// Like `spawn_router`, also handing what the peer says about itself to `on_update`.
    pub fn spawn_router_with(
        &self,
        stream_buffer_size: usize,
        on_update: Option<OnPeerUpdate>,
    ) -> mpsc::Receiver<IncomingCapabilityStream> {
        let (tx, rx) = mpsc::channel(stream_buffer_size);
        let conn = self.connection.clone();
        let shutdown = self.shutdown_notify.clone();
        let keepalive = self.keepalive.clone();
        let peer_disconnected = self.peer_disconnected.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    incoming = conn.accept_bi() => {
                        match incoming {
                            Ok((send, mut recv)) => {
                                match read_capability_stream_header(&mut recv).await {
                                    Ok(CapabilityId::CONTROL) => {
                                        tokio::spawn(Self::handle_control_stream(
                                            send,
                                            recv,
                                            conn.clone(),
                                            keepalive.clone(),
                                            peer_disconnected.clone(),
                                            on_update.clone(),
                                        ));
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
                    _ = conn.closed() => break,
                    _ = interval.tick() => {
                        let should_ping = {
                            let tracker = keepalive.lock().await;
                            if tracker.is_timed_out() {
                                warn!("Keepalive timed out for peer session");
                                conn.close(close_code(DisconnectReason::Error), b"keepalive timeout");
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
        conn: quinn::Connection,
        keepalive: Arc<Mutex<KeepaliveTracker>>,
        peer_disconnected: Arc<AtomicBool>,
        on_update: Option<OnPeerUpdate>,
    ) {
        if let Ok(env) = Session::read_envelope(&mut recv).await {
            match env.body {
                Some(Body::Ping(Ping { seq })) => {
                    let pong = KeepaliveTracker::make_pong(seq);
                    let _ = Session::send_envelope(&mut send, &pong).await;
                    let mut tracker = keepalive.lock().await;
                    tracker.record_activity();
                }
                Some(Body::Disconnect(Disconnect { reason, message })) => {
                    debug!("Peer sent disconnect: reason={reason:?}, message={message}");
                    let reason =
                        DisconnectReason::try_from(reason).unwrap_or(DisconnectReason::Unspecified);
                    if reason == DisconnectReason::Normal {
                        peer_disconnected.store(true, Ordering::Release);
                    }
                    conn.close(close_code(reason), b"peer disconnected");
                }
                Some(Body::DeviceInfo(info)) => {
                    if let Some(on_update) = on_update {
                        on_update(PeerUpdate::Info(info));
                    }
                }
                Some(Body::DeviceStatus(status)) => {
                    if let Some(on_update) = on_update {
                        on_update(PeerUpdate::Status(status));
                    }
                }
                Some(Body::DeviceLook(look)) => {
                    if let Some(on_update) = on_update {
                        on_update(PeerUpdate::Look(look));
                    }
                }
                _ => {}
            }
        }
    }

    /// Tells the peer about this device. Best effort: a peer that misses it keeps what it
    /// had.
    pub async fn send_control(&self, body: Body) {
        let Ok((mut send, _recv)) =
            open_capability_stream(&self.connection, CapabilityId::CONTROL).await
        else {
            return;
        };
        let env = SessionEnvelope { body: Some(body) };
        if Session::send_envelope(&mut send, &env).await.is_ok() {
            let _ = send.finish();
        }
    }

    /// Disconnect session and release resources.
    pub async fn disconnect(&self, reason: DisconnectReason, message: String) {
        self.shutdown_notify.notify_waiters();

        if let Ok((mut send, _)) =
            open_capability_stream(&self.connection, CapabilityId::CONTROL).await
        {
            let env = SessionEnvelope {
                body: Some(Body::Disconnect(Disconnect {
                    reason: reason as i32,
                    message: message.clone(),
                })),
            };
            let _ = Session::send_envelope(&mut send, &env).await;
        }

        self.connection
            .close(close_code(reason), message.as_bytes());
    }
}
