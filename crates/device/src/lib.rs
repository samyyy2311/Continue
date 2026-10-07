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
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

    #[error("Connections are paused")]
    Paused,

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

    #[error(transparent)]
    Notification(#[from] notifications::NotificationError),

    #[error(transparent)]
    Transport(#[from] TransportError),
}

/// A device that finished pairing but isn't trusted until accepted.
pub struct PendingPair {
    device: Device,
    pending: pairing::PendingPeer,
    ip: IpAddr,
}

impl PendingPair {
    /// Six digits the other device shows too; different digits mean it isn't the device meant.
    pub fn code(&self) -> &str {
        &self.pending.code
    }

    /// Trusts the device from now on and connects to it.
    pub fn accept(self) -> Result<TrustedPeer, PairingError> {
        let peer = self.pending.accept()?;
        self.device.paired(&peer, self.ip);
        Ok(peer)
    }
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
    /// Held while sending what was waiting, so a quick reconnect doesn't send it twice.
    sending_waiting: Arc<tokio::sync::Mutex<()>>,
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
        let mut handlers = handlers.with_permission_store(stores.permissions.clone());
        handlers.snippet_store = Some(Arc::new(MergeSnippets {
            history: stores.history.clone(),
            on_changed: handlers.on_snippets_changed.clone(),
        }));
        let sessions = SessionRegistry::new(
            fingerprint.clone(),
            keys.transport_cert.clone(),
            handlers,
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
            sending_waiting: Arc::default(),
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
    /// Pairs from a code read off the other device's screen, which proves which device it is.
    pub async fn pair_with_code(&self, code: &str) -> Result<TrustedPeer, PairError> {
        Ok(self.pair_unconfirmed(code).await?.accept()?)
    }

    /// Pairs from a code found on the network, which anyone nearby could have put there: the
    /// person compares [`PendingPair::code`] with the computer's screen before accepting.
    pub async fn pair_nearby(&self, code: &str) -> Result<PendingPair, PairError> {
        self.pair_unconfirmed(code).await
    }

    async fn pair_unconfirmed(&self, code: &str) -> Result<PendingPair, PairError> {
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
        let pending = pairing::ResponderPairing::new(
            self.keys.identity_signer.clone(),
            self.keys.transport_cert.clone(),
            self.stores.trust.clone(),
        )
        .complete_handshake(&qr, &mut send, &mut recv)
        .await?;
        // Dropping the connection would discard the last message if it hadn't arrived yet.
        let _ = send.finish();
        let _ = tokio::time::timeout(Duration::from_secs(5), send.stopped()).await;
        // The pairing connection closes here; once accepted, the session dials this address.
        Ok(PendingPair {
            device: self.clone(),
            pending,
            ip: addr.ip(),
        })
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
        if let Err(error) = self.stores.history.forget_waiting_for(fingerprint) {
            tracing::warn!("Couldn't drop what was waiting for {fingerprint}: {error}");
        }
        self.sessions.remove(fingerprint).await;
        Ok(removed)
    }

    /// Connects to a paired device at `addr`, and dials it there from now on.
    pub async fn connect(&self, fingerprint: &str, addr: SocketAddr) -> Result<(), ConnectError> {
        if self.sessions.is_paused() {
            return Err(ConnectError::Paused);
        }
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

    /// Sends a notification, or a reply to or dismissal of one.
    pub async fn send_notification(
        &self,
        peer: &str,
        body: notifications::Body,
    ) -> Result<(), SendError> {
        let mux = self.sessions.get(peer).ok_or(SendError::NotConnected)?;
        let query = CapabilityQuery::negotiated(CapabilityId::NOTIFICATIONS, true);
        mux.send_notification_to_peer(body, &query).await?;
        Ok(())
    }

    pub async fn photos(
        &self,
        peer: &str,
        request: protocol::v1::photos_message::Body,
    ) -> Result<protocol::v1::PhotosReply, SendError> {
        let mux = self.sessions.get(peer).ok_or(SendError::NotConnected)?;
        Ok(mux.send_photos_message(request).await?)
    }

    /// Looks through the peer phone's files, texts and contacts.
    pub async fn search(
        &self,
        peer: &str,
        query: String,
    ) -> Result<protocol::v1::SearchReply, SendError> {
        let mux = self.sessions.get(peer).ok_or(SendError::NotConnected)?;
        Ok(mux.search_peer(query).await?)
    }

    /// Has the peer computer lock itself, type something or open a link. False if it didn't.
    pub async fn act(
        &self,
        peer: &str,
        action: protocol::v1::computer_action::Body,
    ) -> Result<bool, SendError> {
        let mux = self.sessions.get(peer).ok_or(SendError::NotConnected)?;
        Ok(mux.act_on_peer(action).await?.done)
    }

    /// Rings the peer even on silent, or stops it.
    pub async fn ring(&self, peer: &str, on: bool) -> Result<bool, SendError> {
        let mux = self.sessions.get(peer).ok_or(SendError::NotConnected)?;
        Ok(mux.ring_peer(on).await?.done)
    }

    pub async fn media(
        &self,
        peer: &str,
        request: protocol::v1::media_message::Body,
    ) -> Result<protocol::v1::MediaReply, SendError> {
        let mux = self.sessions.get(peer).ok_or(SendError::NotConnected)?;
        Ok(mux.send_media_message(request).await?)
    }

    pub async fn calls(
        &self,
        peer: &str,
        request: protocol::v1::calls_message::Body,
    ) -> Result<protocol::v1::CallsReply, SendError> {
        let mux = self.sessions.get(peer).ok_or(SendError::NotConnected)?;
        Ok(mux.send_calls_message(request).await?)
    }

    pub async fn files(
        &self,
        peer: &str,
        request: protocol::v1::files_message::Body,
    ) -> Result<protocol::v1::FilesReply, SendError> {
        let mux = self.sessions.get(peer).ok_or(SendError::NotConnected)?;
        Ok(mux.send_files_message(request).await?)
    }

    pub async fn messages(
        &self,
        peer: &str,
        request: protocol::v1::messages_message::Body,
    ) -> Result<protocol::v1::MessagesReply, SendError> {
        let mux = self.sessions.get(peer).ok_or(SendError::NotConnected)?;
        Ok(mux.send_messages_message(request).await?)
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

    /// Keeps text or a file path for a device that isn't connected, to send with
    /// [`Device::send_waiting`] once it is.
    pub fn send_later(&self, peer: &str, kind: Kind, content: &str) -> Result<i64, DeviceError> {
        Ok(self.stores.history.add_waiting(peer, kind, content)?)
    }

    /// Sends what was kept for `peer`, oldest first, telling `on_done` each item and whether it
    /// went. Stops if the device goes away again, keeping the rest for next time.
    pub async fn send_waiting(&self, peer: &str, on_done: impl Fn(&history::Waiting, bool)) {
        let _one_at_a_time = self.sending_waiting.lock().await;
        let waiting = match self.stores.history.waiting(Some(peer)) {
            Ok(waiting) => waiting,
            Err(error) => return tracing::warn!("Couldn't read what's waiting: {error}"),
        };
        for item in waiting {
            let sent = match item.kind {
                Kind::Text => match self.send_text(peer, item.content.clone()).await {
                    Err(SendError::NotConnected) => return,
                    result => result.is_ok(),
                },
                Kind::File => {
                    let path = Path::new(&item.content);
                    let location = Some(item.content.clone());
                    match self
                        .send_file(peer, path, location, None::<fn(u64, u64)>)
                        .await
                    {
                        Err(TransferError::NotConnected) => return,
                        result => result.is_ok(),
                    }
                }
            };
            // A failed send is in history already; retrying it forever helps nobody.
            if let Err(error) = self.stores.history.remove_waiting(item.id) {
                tracing::warn!("Couldn't clear a sent item from the queue: {error}");
            }
            on_done(&item, sent);
        }
    }

    /// Pins text on every paired device: here now, connected ones straight away and the rest
    /// when they next connect.
    pub async fn pin(&self, text: &str) -> Result<history::Snippet, DeviceError> {
        let snippet = self.stores.history.pin(text)?;
        self.share_snippets_with_connected().await;
        Ok(snippet)
    }

    pub async fn unpin(&self, id: &str) -> Result<(), DeviceError> {
        self.stores.history.unpin(id)?;
        self.share_snippets_with_connected().await;
        Ok(())
    }

    /// Sends every snippet here, removed ones included, so the peer can catch up.
    pub async fn share_snippets(&self, peer: &str) {
        let (Some(mux), Ok(snippets)) =
            (self.sessions.get(peer), self.stores.history.snippets(true))
        else {
            return;
        };
        let snippets = snippets.into_iter().map(to_message).collect();
        if let Err(error) = mux.send_snippets(snippets).await {
            tracing::debug!("Couldn't share snippets with {peer}: {error}");
        }
    }

    async fn share_snippets_with_connected(&self) {
        for peer in self.sessions.connected() {
            self.share_snippets(&peer).await;
        }
    }

    /// Puts a PNG on the peer's clipboard. Images aren't kept in history.
    pub async fn send_image(&self, peer: &str, png: Vec<u8>) -> Result<(), SendError> {
        let mux = self.sessions.get(peer).ok_or(SendError::NotConnected)?;
        let query = CapabilityQuery::negotiated(CapabilityId::CLIPBOARD, true);
        mux.send_clipboard_to_peer(clipboard::ClipboardFormat::ImagePng, png, &query)
            .await?;
        Ok(())
    }
}

struct MergeSnippets {
    history: history::HistoryStore,
    on_changed: Option<sessions::OnReceived<()>>,
}

impl sessions::SnippetStore for MergeSnippets {
    fn merge(&self, peer: &str, snippets: Vec<protocol::v1::Snippet>) {
        let theirs: Vec<_> = snippets
            .into_iter()
            .map(|s| history::Snippet {
                id: s.id,
                text: s.text,
                changed_at: s.changed_at,
                removed: s.removed,
            })
            .collect();
        match self.history.merge_snippets(&theirs) {
            Ok(true) => {
                if let Some(on_changed) = &self.on_changed {
                    on_changed(peer, ());
                }
            }
            Ok(false) => {}
            Err(error) => tracing::warn!("Couldn't keep snippets from {peer}: {error}"),
        }
    }
}

fn to_message(snippet: history::Snippet) -> protocol::v1::Snippet {
    protocol::v1::Snippet {
        id: snippet.id,
        text: snippet.text,
        changed_at: snippet.changed_at,
        removed: snippet.removed,
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}
