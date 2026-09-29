// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tracing::{debug, info};

use limits::CONNECTION_TIMEOUT_SECS;
use protocol::v1::DisconnectReason;
use transport::{connect_pinned, DialConfig, TransportCertificate, TransportError};

use crate::backoff::ReconnectPolicy;
use crate::capabilities_router::{spawn_capabilities_dispatcher, SessionCapabilityHandlers};
use crate::multiplexer::{close_code, SessionMultiplexer};
use crate::session::Session;
use crate::state::SessionState;

const STREAM_BUFFER: usize = 16;

/// Which side opened a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Outbound,
    Inbound,
}

pub type StateListener = Arc<dyn Fn(&str, SessionState) + Send + Sync>;

#[derive(Clone, Default)]
pub struct RegistryConfig {
    pub dial: DialConfig,
    pub reconnect: ReconnectPolicy,
}

struct Entry {
    session: Session,
    direction: Direction,
    established: Instant,
    /// Identifies the current reconnect episode so a superseded loop stops.
    reconnect_episode: u64,
    /// Set when either user ended the session, so discovery doesn't reconnect it.
    ended_on_purpose: bool,
}

struct Inner {
    local_fingerprint: String,
    transport_cert: Arc<TransportCertificate>,
    handlers: SessionCapabilityHandlers,
    config: RegistryConfig,
    on_state_change: Option<StateListener>,
    peers: Mutex<HashMap<String, Entry>>,
}

impl Entry {
    /// Puts `addr` first in the list reconnects try.
    fn remember_address(&mut self, addr: SocketAddr) {
        self.session.known_addresses.retain(|known| *known != addr);
        self.session.known_addresses.insert(0, addr);
    }
}

/// Owns every peer session: connecting, replacing duplicates, noticing when a connection
/// dies and reconnecting after unexpected loss.
#[derive(Clone)]
pub struct SessionRegistry {
    inner: Arc<Inner>,
}

impl SessionRegistry {
    pub fn new(
        local_fingerprint: String,
        transport_cert: Arc<TransportCertificate>,
        handlers: SessionCapabilityHandlers,
        config: RegistryConfig,
        on_state_change: Option<StateListener>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                local_fingerprint,
                transport_cert,
                handlers,
                config,
                on_state_change,
                peers: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// The live multiplexer for a peer, if it is connected.
    pub fn get(&self, peer: &str) -> Option<Arc<SessionMultiplexer>> {
        self.peers()
            .get(peer)
            .and_then(|entry| entry.session.active.clone())
            .filter(|mux| mux.connection().close_reason().is_none())
    }

    pub fn state(&self, peer: &str) -> SessionState {
        self.peers()
            .get(peer)
            .map_or(SessionState::Disconnected, |entry| entry.session.state)
    }

    /// Connect to a trusted peer at `addr`, pinned to its transport key.
    pub async fn connect(
        &self,
        peer: &str,
        spki_hash: [u8; 32],
        addr: SocketAddr,
    ) -> Result<(), TransportError> {
        if self.get(peer).is_some() {
            return Ok(());
        }
        {
            let mut peers = self.peers();
            let entry = self.entry(&mut peers, peer, spki_hash);
            entry.session.peer_transport_spki_hash = spki_hash;
            entry.remember_address(addr);
            entry.session.state = SessionState::Connecting;
        }
        self.notify(peer, SessionState::Connecting);

        match connect_pinned(
            &self.inner.transport_cert,
            spki_hash,
            addr,
            &self.inner.config.dial,
        )
        .await
        {
            Ok(connection) => {
                self.attach(peer, spki_hash, connection, Direction::Outbound);
                Ok(())
            }
            Err(error) => {
                let failed = {
                    let mut peers = self.peers();
                    match peers.get_mut(peer) {
                        Some(entry) if entry.session.state == SessionState::Connecting => {
                            entry.session.state = SessionState::Disconnected;
                            true
                        }
                        _ => false,
                    }
                };
                if failed {
                    self.notify(peer, SessionState::Disconnected);
                }
                Err(error)
            }
        }
    }

    /// Dial a trusted peer at addresses just seen on the network. The advertisement doesn't
    /// say which device it is, so only a pinned handshake with the right peer succeeds and
    /// failures are expected and not reported. Skips peers that are connected, being dialed,
    /// or were disconnected on purpose. Returns the address that worked.
    pub async fn connect_discovered(
        &self,
        peer: &str,
        spki_hash: [u8; 32],
        addresses: &[SocketAddr],
    ) -> Option<SocketAddr> {
        for &addr in addresses {
            if !self.wants_discovered(peer) {
                return None;
            }
            match connect_pinned(
                &self.inner.transport_cert,
                spki_hash,
                addr,
                &self.inner.config.dial,
            )
            .await
            {
                Ok(connection) => {
                    if !self.wants_discovered(peer) {
                        connection.close(close_code(DisconnectReason::Normal), b"not needed");
                        return None;
                    }
                    self.entry(&mut self.peers(), peer, spki_hash)
                        .remember_address(addr);
                    self.attach(peer, spki_hash, connection, Direction::Outbound);
                    return Some(addr);
                }
                Err(error) => debug!("{peer} is not at {addr}: {error}"),
            }
        }
        None
    }

    fn wants_discovered(&self, peer: &str) -> bool {
        self.get(peer).is_none()
            && self.peers().get(peer).is_none_or(|entry| {
                !entry.ended_on_purpose && entry.session.state != SessionState::Connecting
            })
    }

    /// Adopt an authenticated connection as the peer's session. Returns false if an existing
    /// live connection was kept instead and this one was closed.
    pub fn attach(
        &self,
        peer: &str,
        spki_hash: [u8; 32],
        connection: quinn::Connection,
        direction: Direction,
    ) -> bool {
        let mux = Arc::new(SessionMultiplexer::new(peer.to_string(), connection));
        {
            let mut peers = self.peers();
            let entry = self.entry(&mut peers, peer, spki_hash);
            let live = entry
                .session
                .active
                .clone()
                .filter(|existing| existing.connection().close_reason().is_none());
            if let Some(existing) = live {
                if !self.replaces(peer, direction, entry) {
                    debug!("Keeping the existing connection to {peer}; closing the new one");
                    mux.connection()
                        .close(close_code(DisconnectReason::Normal), b"duplicate");
                    return false;
                }
                existing
                    .connection()
                    .close(close_code(DisconnectReason::Normal), b"duplicate");
            }
            entry.session.peer_transport_spki_hash = spki_hash;
            entry.session.attach_connection(mux.clone());
            entry.direction = direction;
            entry.established = Instant::now();
            entry.ended_on_purpose = false;
        }

        mux.spawn_keepalive_sender();
        spawn_capabilities_dispatcher(mux.clone(), self.inner.handlers.clone(), STREAM_BUFFER);
        self.watch(peer.to_string(), mux);
        self.notify(peer, SessionState::Connected);
        info!("Session established with {peer} ({direction:?})");
        true
    }

    /// Disconnect at the user's request. No reconnect follows.
    pub async fn disconnect(&self, peer: &str) {
        let mux = {
            let mut peers = self.peers();
            let Some(entry) = peers.get_mut(peer) else {
                return;
            };
            let mux = entry.session.active.take();
            entry.session.close();
            entry.ended_on_purpose = true;
            mux
        };
        self.clear_connection_grants(peer);
        self.notify(peer, SessionState::Closed);
        if let Some(mux) = mux {
            mux.disconnect(DisconnectReason::Normal, "Disconnected by user".to_string())
                .await;
        }
    }

    /// Disconnect and forget the peer, e.g. after unpairing.
    pub async fn remove(&self, peer: &str) {
        self.disconnect(peer).await;
        self.peers().remove(peer);
    }

    fn peers(&self) -> MutexGuard<'_, HashMap<String, Entry>> {
        self.inner.peers.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn entry<'a>(
        &self,
        peers: &'a mut HashMap<String, Entry>,
        peer: &str,
        spki_hash: [u8; 32],
    ) -> &'a mut Entry {
        peers.entry(peer.to_string()).or_insert_with(|| {
            let mut session = Session::new(peer.to_string(), spki_hash, Vec::new());
            session.reconnect_policy = self.inner.config.reconnect.clone();
            Entry {
                session,
                direction: Direction::Outbound,
                established: Instant::now(),
                reconnect_episode: 0,
                ended_on_purpose: false,
            }
        })
    }

    /// Decides between a live connection and a new one to the same peer. When both peers dial
    /// at once, each side keeps the connection opened by the lower fingerprint, so both ends
    /// agree without exchanging anything. Otherwise the new connection wins: a peer only dials
    /// again when it has lost the old one, even if this side has not noticed yet.
    fn replaces(&self, peer: &str, direction: Direction, existing: &Entry) -> bool {
        let simultaneous = direction != existing.direction
            && existing.established.elapsed() < Duration::from_secs(CONNECTION_TIMEOUT_SECS);
        if !simultaneous {
            return true;
        }
        self.dialer(peer, direction) < self.dialer(peer, existing.direction)
    }

    fn dialer<'a>(&'a self, peer: &'a str, direction: Direction) -> &'a str {
        match direction {
            Direction::Outbound => &self.inner.local_fingerprint,
            Direction::Inbound => peer,
        }
    }

    fn watch(&self, peer: String, mux: Arc<SessionMultiplexer>) {
        let registry = self.clone();
        tokio::spawn(async move {
            let error = mux.connection().closed().await;
            registry.connection_closed(&peer, &mux, &error);
        });
    }

    fn connection_closed(
        &self,
        peer: &str,
        mux: &Arc<SessionMultiplexer>,
        error: &quinn::ConnectionError,
    ) {
        let (state, reconnect) = {
            let mut peers = self.peers();
            let Some(entry) = peers.get_mut(peer) else {
                return;
            };
            let current = entry
                .session
                .active
                .as_ref()
                .is_some_and(|active| Arc::ptr_eq(active, mux));
            if !current {
                return;
            }
            if mux.ended_by_peer(error) || entry.session.known_addresses.is_empty() {
                entry.ended_on_purpose = mux.ended_by_peer(error);
                entry.session.active = None;
                entry.session.state = SessionState::Disconnected;
                (SessionState::Disconnected, None)
            } else {
                let delay = entry.session.mark_disconnected();
                entry.reconnect_episode += 1;
                let episode = entry.reconnect_episode;
                (entry.session.state, delay.map(|delay| (delay, episode)))
            }
        };

        info!("Connection to {peer} ended: {error}");
        self.clear_connection_grants(peer);
        self.notify(peer, state);
        if let Some((delay, episode)) = reconnect {
            tokio::spawn(self.clone().reconnect(peer.to_string(), delay, episode));
        }
    }

    async fn reconnect(self, peer: String, mut delay: Duration, episode: u64) {
        loop {
            tokio::time::sleep(delay).await;
            let Some((spki_hash, addresses)) = self.reconnect_targets(&peer, episode) else {
                return;
            };
            for addr in addresses {
                match connect_pinned(
                    &self.inner.transport_cert,
                    spki_hash,
                    addr,
                    &self.inner.config.dial,
                )
                .await
                {
                    Ok(connection) => {
                        if self.reconnect_targets(&peer, episode).is_some() {
                            self.attach(&peer, spki_hash, connection, Direction::Outbound);
                        } else {
                            connection.close(close_code(DisconnectReason::Normal), b"not needed");
                        }
                        return;
                    }
                    Err(error) => debug!("Reconnect to {peer} at {addr} failed: {error}"),
                }
            }

            let next = {
                let mut peers = self.peers();
                match peers.get_mut(&peer) {
                    Some(entry)
                        if entry.session.state == SessionState::Reconnecting
                            && entry.reconnect_episode == episode =>
                    {
                        let next = entry.session.reconnect_policy.next_delay();
                        if next.is_none() {
                            entry.session.state = SessionState::Disconnected;
                        }
                        next
                    }
                    _ => return,
                }
            };
            match next {
                Some(next) => delay = next,
                None => {
                    info!("Giving up reconnecting to {peer}");
                    self.notify(&peer, SessionState::Disconnected);
                    return;
                }
            }
        }
    }

    /// Where to redial, or `None` once this episode no longer needs to reconnect.
    fn reconnect_targets(&self, peer: &str, episode: u64) -> Option<([u8; 32], Vec<SocketAddr>)> {
        self.peers()
            .get(peer)
            .filter(|entry| {
                entry.session.state == SessionState::Reconnecting
                    && entry.reconnect_episode == episode
            })
            .map(|entry| {
                (
                    entry.session.peer_transport_spki_hash,
                    entry.session.known_addresses.clone(),
                )
            })
    }

    fn clear_connection_grants(&self, peer: &str) {
        if let Some(store) = &self.inner.handlers.permission_store {
            store.clear_allow_once_for_peer(peer);
        }
    }

    fn notify(&self, peer: &str, state: SessionState) {
        if let Some(listener) = &self.inner.on_state_change {
            listener(peer, state);
        }
    }
}
