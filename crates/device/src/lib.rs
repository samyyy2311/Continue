// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! A running Continue device: its keys, what it has saved, and its sessions with paired
//! devices. The desktop app and the phone's core each run one.

mod pairing_server;
mod stores;

use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use capabilities::CapabilityQuery;
use history::{Direction, Kind};
use pairing::{DeviceKeys, InitiatorPairing, PairingError, ReplayCache, TrustedPeer};
use protocol::CapabilityId;
use sessions::{SessionCapabilityHandlers, SessionRegistry, StateListener};
use thiserror::Error;
use tokio::task::JoinHandle;
use transfer::TransferError;
use transport::{DialConfig, TransportError};

pub use pairing_server::PairingServer;
pub use stores::{Recorded, Stores};

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error(transparent)]
    Pairing(#[from] PairingError),

    #[error(transparent)]
    Permissions(#[from] permissions::PermissionError),

    #[error(transparent)]
    History(#[from] history::HistoryError),

    #[error(transparent)]
    Signer(#[from] identity::SignerError),
}

/// Why pairing from a code another device shows didn't work.
#[derive(Debug, Error)]
pub enum PairError {
    #[error("{0}")]
    BadCode(String),

    #[error("{0}")]
    Unreachable(String),

    #[error(transparent)]
    Failed(#[from] PairingError),
}

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("Peer not found in trust store")]
    NotPaired,

    #[error(transparent)]
    Store(#[from] PairingError),

    #[error("Connect failed: {0}")]
    Unreachable(#[from] TransportError),
}

#[derive(Debug, Error)]
pub enum SendError {
    #[error("Peer not connected")]
    NotConnected,

    #[error(transparent)]
    Clipboard(#[from] clipboard::ClipboardError),
}

/// Cheap to clone; every clone is the same device.
#[derive(Clone)]
pub struct Device {
    pub stores: Stores,
    pub sessions: SessionRegistry,
    pub keys: DeviceKeys,
    pub fingerprint: String,
    /// Transport keys of paired devices; only these get through the listener's handshake.
    trusted_keys: Arc<RwLock<HashSet<[u8; 32]>>>,
    replay_cache: Arc<ReplayCache>,
}

impl Device {
    pub fn new(
        stores: Stores,
        keys: DeviceKeys,
        handlers: SessionCapabilityHandlers,
        on_state_change: Option<StateListener>,
    ) -> Result<Self, DeviceError> {
        let fingerprint =
            identity::Fingerprint::from_verifying_key(&keys.identity_signer.verifying_key()?)
                .to_string();
        let trusted_keys = stores
            .trust
            .list_peers()?
            .into_iter()
            .map(|peer| peer.transport_spki_hash)
            .collect();
        let sessions = SessionRegistry::new(
            fingerprint.clone(),
            keys.transport_cert.clone(),
            handlers.with_permission_store(stores.permissions.clone()),
            sessions::RegistryConfig::default(),
            on_state_change,
        );
        Ok(Self {
            stores,
            sessions,
            keys,
            fingerprint,
            trusted_keys: Arc::new(RwLock::new(trusted_keys)),
            replay_cache: Arc::new(ReplayCache::new()),
        })
    }

    /// Opens the endpoint paired devices connect to and accepts them on it until it is
    /// closed. Must be called inside the Tokio runtime the device runs on.
    pub fn listen(&self) -> Result<quinn::Endpoint, TransportError> {
        let endpoint =
            sessions::listen_for_peers(&self.keys.transport_cert, self.trusted_keys.clone())?;
        tokio::spawn(sessions::accept_peers(
            endpoint.clone(),
            self.sessions.clone(),
            self.stores.trust.clone(),
        ));
        Ok(endpoint)
    }

    /// Advertises the listener on `port` and dials paired devices as they appear, until
    /// the returned tasks are aborted.
    pub fn discover(&self, port: u16, protocol_version: u32) -> [JoinHandle<()>; 2] {
        [
            tokio::spawn(discovery::advertise(port, protocol_version)),
            tokio::spawn(sessions::connect_paired_peers(
                self.sessions.clone(),
                self.stores.trust.clone(),
            )),
        ]
    }

    /// The side of pairing that shows a code.
    pub fn initiator(&self) -> InitiatorPairing {
        InitiatorPairing::new(
            self.keys.identity_signer.clone(),
            self.keys.transport_cert.clone(),
            self.stores.trust.clone(),
            self.replay_cache.clone(),
        )
    }

    /// Pairs with the device showing `code`, then connects to it.
    pub async fn pair_with_code(&self, code: &str) -> Result<TrustedPeer, PairError> {
        let qr = pairing::QrPayload::decode(code).map_err(|e| PairError::BadCode(e.to_string()))?;
        let addr: SocketAddr = qr
            .endpoint
            .parse()
            .map_err(|e| PairError::BadCode(format!("Invalid endpoint address: {e}")))?;
        let connection = transport::connect_pinned(
            &self.keys.transport_cert,
            qr.transport_spki_hash,
            addr,
            &DialConfig::default(),
        )
        .await
        .map_err(|e| PairError::Unreachable(format!("Connect failed: {e}")))?;
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .map_err(|e| PairError::Unreachable(format!("Stream open failed: {e}")))?;
        let peer = pairing::ResponderPairing::new(
            self.keys.identity_signer.clone(),
            self.keys.transport_cert.clone(),
            self.stores.trust.clone(),
        )
        .complete_handshake(&qr, &mut send, &mut recv)
        .await?;
        // The pairing connection closes here; the session dials the address just saved.
        self.paired(&peer, addr.ip());
        Ok(peer)
    }

    /// Lets a newly paired device through the listener and dials it at `ip`.
    fn paired(&self, peer: &TrustedPeer, ip: IpAddr) {
        self.trusted_keys
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(peer.transport_spki_hash);
        sessions::remember_peer_address(&self.stores.trust, &peer.fingerprint, ip);
        self.sessions.redial_now();
    }

    /// Unpairs a device and ends its session. False if it wasn't paired.
    pub async fn forget(&self, fingerprint: &str) -> Result<bool, PairingError> {
        // Revoked first, so the device can't reconnect before its session is gone.
        if let Ok(Some(peer)) = self.stores.trust.get_peer(fingerprint) {
            self.trusted_keys
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&peer.transport_spki_hash);
        }
        let removed = self.stores.trust.remove_peer(fingerprint)?;
        self.sessions.remove(fingerprint).await;
        Ok(removed)
    }

    /// Connects to a paired device at `addr`, and dials it there from now on.
    pub async fn connect(&self, fingerprint: &str, addr: SocketAddr) -> Result<(), ConnectError> {
        let peer = self
            .stores
            .trust
            .get_peer(fingerprint)?
            .ok_or(ConnectError::NotPaired)?;
        self.sessions
            .connect(fingerprint, peer.transport_spki_hash, addr)
            .await?;
        // The connection is up either way; the address only matters next time.
        if let Err(error) = self
            .stores
            .trust
            .set_last_endpoint(fingerprint, &addr.to_string())
        {
            tracing::warn!("Couldn't save the address for {fingerprint}: {error}");
        }
        Ok(())
    }

    /// Sends a file and saves it to history, with `location` as where it stays on this
    /// device. If the connection drops part way, it waits for it to come back and sends the
    /// rest.
    pub async fn send_file(
        &self,
        peer: &str,
        path: &Path,
        location: Option<String>,
        on_progress: Option<impl Fn(u64, u64)>,
    ) -> Result<u64, TransferError> {
        let transfer_id = format!("tx-{}", unix_ms());
        let result = self
            .sessions
            .send_file(peer, path, transfer_id, on_progress)
            .await;
        self.stores.remember(history::Item {
            direction: Direction::Sent,
            kind: Kind::File,
            label: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            peer_fingerprint: peer.to_string(),
            peer_name: self.stores.peer_name(peer),
            size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
            failed: result.is_err(),
            location,
        });
        result
    }

    /// Sends text to the peer's clipboard and saves it to history.
    pub async fn send_text(&self, peer: &str, text: String) -> Result<(), SendError> {
        let mux = self.sessions.get(peer).ok_or(SendError::NotConnected)?;
        let result = mux
            .send_clipboard_to_peer(
                clipboard::ClipboardFormat::TextPlain,
                text.clone().into_bytes(),
                &CapabilityQuery::negotiated(CapabilityId::CLIPBOARD, true),
            )
            .await;
        self.stores.remember(history::Item {
            direction: Direction::Sent,
            kind: Kind::Text,
            size: text.len() as u64,
            label: text,
            peer_fingerprint: peer.to_string(),
            peer_name: self.stores.peer_name(peer),
            failed: result.is_err(),
            location: None,
        });
        result?;
        Ok(())
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}
