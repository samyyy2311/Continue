// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

// Generated UniFFI scaffolding (uniffi 0.28) leaves blank lines after doc comments.
#![allow(clippy::empty_line_after_doc_comments)]

uniffi::include_scaffolding!("continue");

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::Duration;
use thiserror::Error;

use history::{Direction, HistoryStore, Kind};
use identity::{FileSecretStore, IdentitySigner};
use pairing::{DeviceKeys, InitiatorPairing, ReplayCache, TrustStore, TrustedPeer};
use permissions::{PermissionStore, PersistedGrant};
use protocol::CapabilityId;
use sessions::{PermissionDecision, SessionMultiplexer, SessionRegistry, SessionState};
use transport::{DialConfig, TransportCertificate};

#[derive(Debug, Error)]
pub enum ContinueFfiError {
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Invalid QR payload: {0}")]
    InvalidQr(String),

    #[error("Pairing failed: {0}")]
    PairingFailed(String),

    #[error("Pairing timed out")]
    PairingTimeout,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Core has not been initialized")]
    NotInitialized,
}

/// What the app's secret store reports when it can't do what was asked.
#[derive(Debug, Error)]
pub enum SecretStoreFfiError {
    #[error("{reason}")]
    Unavailable { reason: String },
}

impl From<uniffi::UnexpectedUniFFICallbackError> for SecretStoreFfiError {
    fn from(error: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Unavailable {
            reason: error.reason,
        }
    }
}

/// The platform's secret store, implemented by the app.
pub trait SecretStoreFfi: Send + Sync {
    fn store(&self, label: String, secret: Vec<u8>) -> Result<(), SecretStoreFfiError>;
    fn load(&self, label: String) -> Result<Option<Vec<u8>>, SecretStoreFfiError>;
    fn delete(&self, label: String) -> Result<(), SecretStoreFfiError>;
}

/// Set by the app before `init_core`; the keys stay in files until it is.
static KEY_STORE: Mutex<Option<Arc<dyn SecretStoreFfi>>> = Mutex::new(None);

pub fn set_key_store(store: Box<dyn SecretStoreFfi>) {
    *KEY_STORE.lock().unwrap() = Some(Arc::from(store));
}

/// The app's store, seen as the core's `SecretStore`.
struct AppSecretStore(Arc<dyn SecretStoreFfi>);

fn from_app(error: SecretStoreFfiError) -> identity::SecretStoreError {
    identity::SecretStoreError::Unavailable(error.to_string())
}

impl identity::SecretStore for AppSecretStore {
    fn store(
        &self,
        label: &str,
        secret: zeroize::Zeroizing<Vec<u8>>,
    ) -> Result<(), identity::SecretStoreError> {
        self.0
            .store(label.to_string(), secret.to_vec())
            .map_err(from_app)
    }

    fn load(
        &self,
        label: &str,
    ) -> Result<Option<zeroize::Zeroizing<Vec<u8>>>, identity::SecretStoreError> {
        Ok(self
            .0
            .load(label.to_string())
            .map_err(from_app)?
            .map(zeroize::Zeroizing::new))
    }

    fn delete(&self, label: &str) -> Result<(), identity::SecretStoreError> {
        self.0.delete(label.to_string()).map_err(from_app)
    }
}

struct ActivePairing {
    server_endpoint: quinn::Endpoint,
    result_rx: tokio::sync::oneshot::Receiver<Result<TrustedPeer, pairing::PairingError>>,
}

struct CoreState {
    runtime: Arc<tokio::runtime::Runtime>,
    trust_store: TrustStore,
    permission_store: PermissionStore,
    history: HistoryStore,
    transport_cert: Arc<TransportCertificate>,
    identity_signer: Arc<dyn IdentitySigner>,
    replay_cache: Arc<ReplayCache>,
    active_pairing: Option<ActivePairing>,
    sessions: SessionRegistry,
    /// Transport keys of paired devices; only these may connect to `listener`.
    trusted_keys: Arc<RwLock<HashSet<[u8; 32]>>>,
    listener: quinn::Endpoint,
    discovery_tasks: Vec<tokio::task::JoinHandle<()>>,
    incoming: sessions::IncomingFiles,
    /// The name paired devices see for this phone.
    this_device: sessions::ThisDevice,
}

static CORE: Mutex<Option<CoreState>> = Mutex::new(None);

/// Something a device set to Ask wants to send, for the app to put to the user.
pub struct PermissionRequestFfi {
    pub id: u64,
    pub peer_fingerprint: String,
    pub peer_name: String,
    pub capability_id: u32,
    /// The file name, for files.
    pub detail: Option<String>,
    /// Unix time in milliseconds when the core declines an unanswered question.
    pub expires_at: u64,
}

pub enum PermissionDecisionFfi {
    Allow,
    AlwaysAllow,
    Decline,
}

/// A file or text a paired device sent to this phone. Files arrive as `file_path` and
/// `file_name`, text as `text`.
pub struct ReceivedFfi {
    /// The history entry, for `set_history_location` once the app has moved the file.
    pub history_id: Option<i64>,
    pub peer_fingerprint: String,
    pub peer_name: String,
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub size: u64,
    pub text: Option<String>,
}

/// Items for the app to pick up whenever it next asks. Kept outside `CORE` so waiting on one
/// never holds up other calls.
struct Inbox<T> {
    items: Mutex<VecDeque<T>>,
    arrived: Condvar,
}

impl<T> Inbox<T> {
    const fn new() -> Self {
        Self {
            items: Mutex::new(VecDeque::new()),
            arrived: Condvar::new(),
        }
    }

    /// Adds an item, dropping the oldest beyond `limit` so an app that stops listening
    /// doesn't grow the queue forever.
    fn push(&self, item: T, limit: usize) {
        let mut items = self.items.lock().unwrap();
        items.push_back(item);
        while items.len() > limit {
            items.pop_front();
        }
        self.arrived.notify_all();
    }

    fn next(&self, timeout: Duration) -> Option<T> {
        let items = self.items.lock().unwrap();
        let (mut items, _) = self
            .arrived
            .wait_timeout_while(items, timeout, |items| items.is_empty())
            .unwrap();
        items.pop_front()
    }
}

/// Questions waiting for the app to pick up, and the answers the core is waiting on.
struct Questions {
    waiting: Inbox<PermissionRequestFfi>,
    answers: Mutex<BTreeMap<u64, tokio::sync::oneshot::Sender<PermissionDecision>>>,
    next_id: AtomicU64,
}

static QUESTIONS: Questions = Questions {
    waiting: Inbox::new(),
    answers: Mutex::new(BTreeMap::new()),
    next_id: AtomicU64::new(0),
};

static RECEIVED: Inbox<ReceivedFfi> = Inbox::new();
const RECEIVED_LIMIT: usize = 100;

fn peer_name(trust_store: &TrustStore, fingerprint: &str) -> String {
    trust_store
        .get_peer(fingerprint)
        .ok()
        .flatten()
        .map(|p| p.display_name)
        .unwrap_or_default()
}

/// Saves to history; a failure there shouldn't fail what was actually done.
fn remember(history: &HistoryStore, item: history::Item) -> Option<i64> {
    history
        .record(&item)
        .map_err(|error| tracing::warn!("Couldn't save to history: {error}"))
        .ok()
}

/// Saves received files and text to history and hands them to the app through
/// `next_received`.
fn deliver_received(
    mut handlers: sessions::SessionCapabilityHandlers,
    trust_store: TrustStore,
    history: HistoryStore,
) -> sessions::SessionCapabilityHandlers {
    let names = trust_store.clone();
    handlers.on_device_info = Some(Arc::new(move |peer, device| {
        if let Err(error) = names.set_display_name(peer, &device.name) {
            tracing::warn!("Couldn't save the name of {peer}: {error}");
        }
    }));
    let (peers, saved) = (trust_store.clone(), history.clone());
    handlers.on_file_received = Some(Arc::new(move |peer, file| {
        let peer_name = peer_name(&peers, peer);
        let history_id = remember(
            &saved,
            history::Item {
                direction: Direction::Received,
                kind: Kind::File,
                label: file.file_name.clone(),
                peer_fingerprint: peer.to_string(),
                peer_name: peer_name.clone(),
                size: file.bytes_received,
                failed: false,
                location: None,
            },
        );
        RECEIVED.push(
            ReceivedFfi {
                history_id,
                peer_fingerprint: peer.to_string(),
                peer_name,
                file_path: Some(file.path.to_string_lossy().into_owned()),
                file_name: Some(file.file_name),
                size: file.bytes_received,
                text: None,
            },
            RECEIVED_LIMIT,
        );
    }));
    handlers.on_clipboard_received = Some(Arc::new(move |peer, update| {
        let peer_name = peer_name(&trust_store, peer);
        let text = String::from_utf8_lossy(&update.payload).into_owned();
        let size = update.payload.len() as u64;
        let history_id = remember(
            &history,
            history::Item {
                direction: Direction::Received,
                kind: Kind::Text,
                label: text.clone(),
                peer_fingerprint: peer.to_string(),
                peer_name: peer_name.clone(),
                size,
                failed: false,
                location: None,
            },
        );
        RECEIVED.push(
            ReceivedFfi {
                history_id,
                peer_fingerprint: peer.to_string(),
                peer_name,
                file_path: None,
                file_name: None,
                size,
                text: Some(text),
            },
            RECEIVED_LIMIT,
        );
    }));
    handlers
}

pub struct HistoryEntryFfi {
    pub id: i64,
    /// Unix time in milliseconds.
    pub at: u64,
    pub received: bool,
    pub is_text: bool,
    pub label: String,
    pub peer_fingerprint: String,
    pub peer_name: String,
    pub size: u64,
    pub failed: bool,
    pub location: Option<String>,
}

fn history_store() -> Result<HistoryStore, ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
    Ok(state.history.clone())
}

fn history_error(error: history::HistoryError) -> ContinueFfiError {
    ContinueFfiError::DatabaseError(error.to_string())
}

/// What this phone sent and received, newest first.
pub fn list_history(limit: u32) -> Result<Vec<HistoryEntryFfi>, ContinueFfiError> {
    let entries = history_store()?.list(limit).map_err(history_error)?;
    Ok(entries
        .into_iter()
        .map(|entry| HistoryEntryFfi {
            id: entry.id,
            at: entry.at,
            received: entry.item.direction == Direction::Received,
            is_text: entry.item.kind == Kind::Text,
            label: entry.item.label,
            peer_fingerprint: entry.item.peer_fingerprint,
            peer_name: entry.item.peer_name,
            size: entry.item.size,
            failed: entry.item.failed,
            location: entry.item.location,
        })
        .collect())
}

pub fn clear_history() -> Result<(), ContinueFfiError> {
    history_store()?.clear().map_err(history_error)
}

/// Notes where the app put a received file.
pub fn set_history_location(id: i64, location: String) -> Result<(), ContinueFfiError> {
    history_store()?
        .set_location(id, &location)
        .map_err(history_error)
}

/// Waits up to `timeout_ms` for the next file or text a paired device sent.
pub fn next_received(timeout_ms: u32) -> Option<ReceivedFfi> {
    RECEIVED.next(Duration::from_millis(timeout_ms.into()))
}

fn permission_prompt(trust_store: TrustStore) -> sessions::PermissionPrompt {
    Arc::new(move |request| {
        let (answer, decision) = tokio::sync::oneshot::channel();
        let id = QUESTIONS.next_id.fetch_add(1, Ordering::Relaxed);
        QUESTIONS.answers.lock().unwrap().insert(id, answer);
        QUESTIONS.waiting.push(
            PermissionRequestFfi {
                id,
                peer_name: peer_name(&trust_store, &request.peer),
                peer_fingerprint: request.peer,
                capability_id: request.capability.raw(),
                detail: request.detail,
                expires_at: unix_ms(request.deadline),
            },
            usize::MAX,
        );

        // The core stops waiting at the deadline; drop the question then too.
        tokio::spawn(async move {
            tokio::time::sleep_until(request.deadline).await;
            QUESTIONS.answers.lock().unwrap().remove(&id);
            QUESTIONS
                .waiting
                .items
                .lock()
                .unwrap()
                .retain(|q| q.id != id);
        });
        decision
    })
}

fn unix_ms(deadline: tokio::time::Instant) -> u64 {
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    (std::time::SystemTime::now() + remaining)
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Waits up to `timeout_ms` for the next question for the user.
pub fn next_permission_request(timeout_ms: u32) -> Option<PermissionRequestFfi> {
    QUESTIONS
        .waiting
        .next(Duration::from_millis(timeout_ms.into()))
}

/// Answers a question from `next_permission_request`. Late answers are ignored.
pub fn answer_permission_request(id: u64, decision: PermissionDecisionFfi) {
    let decision = match decision {
        PermissionDecisionFfi::Allow => PermissionDecision::Allow,
        PermissionDecisionFfi::AlwaysAllow => PermissionDecision::AlwaysAllow,
        PermissionDecisionFfi::Decline => PermissionDecision::Decline,
    };
    if let Some(waiting) = QUESTIONS.answers.lock().unwrap().remove(&id) {
        let _ = waiting.send(decision);
    }
}

pub struct TrustedPeerFfi {
    pub fingerprint: String,
    pub display_name: String,
    pub paired_at: u64,
}

impl From<TrustedPeer> for TrustedPeerFfi {
    fn from(p: TrustedPeer) -> Self {
        Self {
            fingerprint: p.fingerprint,
            display_name: p.display_name,
            paired_at: p.paired_at,
        }
    }
}

pub fn init_core(db_path: String) -> Result<(), ContinueFfiError> {
    // Shut the previous core down first so its listener gives up the port.
    let previous = CORE.lock().unwrap().take();
    if let Some(mut previous) = previous {
        stop_tasks(&mut previous.discovery_tasks);
        previous.listener.close(0u32.into(), b"restarting");
    }

    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?,
    );

    let trust_store =
        TrustStore::open(&db_path).map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?;
    let history = HistoryStore::open(&db_path).map_err(history_error)?;
    let permission_store = PermissionStore::open(&db_path)
        .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?;

    let DeviceKeys {
        identity_signer,
        transport_cert,
    } = load_device_keys(&db_path)?;

    let local_fingerprint = identity::Fingerprint::from_verifying_key(
        &identity_signer
            .verifying_key()
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?,
    )
    .to_string();
    // Next to the database, in the app's own storage; the app moves files on from there.
    let download_dir = std::path::Path::new(&db_path)
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("received");
    let _ = std::fs::create_dir_all(&download_dir);
    let grants = permission_store.clone();
    let incoming = sessions::IncomingFiles::default();
    // Named by `set_device_name` once the app has read the phone's name.
    let this_device = sessions::ThisDevice::default();
    let sessions = SessionRegistry::new(
        local_fingerprint,
        transport_cert.clone(),
        deliver_received(
            sessions::SessionCapabilityHandlers::new(download_dir)
                .with_this_device(this_device.clone())
                .with_incoming(incoming.clone())
                .with_permission_store(Arc::new(permission_store.clone()))
                .with_permission_prompt(permission_prompt(trust_store.clone())),
            trust_store.clone(),
            history.clone(),
        ),
        sessions::RegistryConfig::default(),
        Some(Arc::new(move |peer, session_state| {
            if matches!(
                session_state,
                SessionState::Disconnected | SessionState::Closed | SessionState::Reconnecting
            ) {
                grants.clear_allow_once_for_peer(peer);
            }
        })),
    );

    let paired_keys = trust_store
        .list_peers()
        .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?
        .into_iter()
        .map(|peer| peer.transport_spki_hash)
        .collect();
    let trusted_keys = Arc::new(RwLock::new(paired_keys));
    let listener = {
        let _runtime = runtime.enter();
        sessions::listen_for_peers(&transport_cert, trusted_keys.clone())
            .map_err(|e| ContinueFfiError::InternalError(format!("Could not listen: {e}")))?
    };
    runtime.spawn(sessions::accept_peers(
        listener.clone(),
        sessions.clone(),
        trust_store.clone(),
    ));

    let state = CoreState {
        runtime,
        trust_store,
        permission_store,
        history,
        transport_cert,
        identity_signer,
        replay_cache: Arc::new(ReplayCache::new()),
        active_pairing: None,
        sessions,
        trusted_keys,
        listener,
        incoming,
        discovery_tasks: Vec::new(),
        this_device,
    };

    let mut lock = CORE.lock().unwrap();
    *lock = Some(state);
    Ok(())
}

/// Keys live in the app's secret store (the Android Keystore) so pairings survive a restart.
/// Keys from earlier versions, in a `secrets` folder beside the database, move there; without
/// a store they stay in that folder. An in-memory database (used by tests) gets throwaway keys.
fn load_device_keys(db_path: &str) -> Result<DeviceKeys, ContinueFfiError> {
    let internal = |e: String| ContinueFfiError::InternalError(e);

    if db_path == ":memory:" {
        let seed = crypto::keys::generate_ed25519_seed();
        let signing_key = crypto::keys::signing_key_from_seed(&seed.0);
        let transport_cert =
            TransportCertificate::generate().map_err(|e| internal(e.to_string()))?;
        return Ok(DeviceKeys {
            identity_signer: Arc::new(identity::InMemorySigner::new(signing_key)),
            transport_cert: Arc::new(transport_cert),
        });
    }

    let secrets_dir = std::path::Path::new(db_path)
        .parent()
        .ok_or_else(|| internal(format!("Database path has no folder: {db_path}")))?
        .join("secrets");
    let files = FileSecretStore::new(secrets_dir).map_err(|e| internal(e.to_string()))?;
    let app_store = KEY_STORE.lock().unwrap().clone();
    match app_store {
        Some(app_store) => DeviceKeys::load_or_create(&identity::PlatformFirstStore::new(
            Box::new(AppSecretStore(app_store)),
            files,
        )),
        None => DeviceKeys::load_or_create(&files),
    }
    .map_err(|e| internal(e.to_string()))
}

/// Sets the name paired devices see for this phone, from the next session on.
pub fn set_device_name(name: String) -> Result<(), ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
    state.this_device.set_name(&name);
    Ok(())
}

pub fn get_device_fingerprint() -> Result<String, ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
    let key = state
        .identity_signer
        .verifying_key()
        .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;
    Ok(identity::Fingerprint::from_verifying_key(&key)
        .as_str()
        .to_string())
}

/// A file on its way in.
pub struct IncomingFileFfi {
    pub transfer_id: String,
    pub peer_name: String,
    pub file_name: String,
    pub received: u64,
    pub total: u64,
}

/// Files coming in right now, oldest first, for the app to show while they arrive.
pub fn list_incoming() -> Vec<IncomingFileFfi> {
    let lock = CORE.lock().unwrap();
    let Some(state) = lock.as_ref() else {
        return Vec::new();
    };
    let files = state.incoming.list();
    files
        .into_iter()
        .map(|file| IncomingFileFfi {
            peer_name: peer_name(&state.trust_store, &file.peer),
            transfer_id: file.transfer_id,
            file_name: file.file_name,
            received: file.received,
            total: file.total,
        })
        .collect()
}

/// Stops a file part way. False if it already finished or never started.
pub fn cancel_incoming(transfer_id: String) -> bool {
    let lock = CORE.lock().unwrap();
    lock.as_ref()
        .is_some_and(|state| state.incoming.cancel(&transfer_id))
}

pub fn get_device_spki_hash() -> Result<String, ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
    Ok(hex_encode(state.transport_cert.spki_hash))
}

/// Advertises this device's listener and connects to paired devices as they appear on
/// the network, until `stop_discovery`.
pub fn start_discovery(protocol_version: u32) -> Result<(), ContinueFfiError> {
    let mut lock = CORE.lock().unwrap();
    let state = lock.as_mut().ok_or(ContinueFfiError::NotInitialized)?;
    let port = state
        .listener
        .local_addr()
        .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?
        .port();

    stop_tasks(&mut state.discovery_tasks);
    state.discovery_tasks = vec![
        state
            .runtime
            .spawn(discovery::advertise(port, protocol_version)),
        state.runtime.spawn(sessions::connect_paired_peers(
            state.sessions.clone(),
            state.trust_store.clone(),
        )),
    ];
    Ok(())
}

pub fn stop_discovery() -> Result<(), ContinueFfiError> {
    let mut lock = CORE.lock().unwrap();
    let state = lock.as_mut().ok_or(ContinueFfiError::NotInitialized)?;
    stop_tasks(&mut state.discovery_tasks);
    Ok(())
}

fn stop_tasks(tasks: &mut Vec<tokio::task::JoinHandle<()>>) {
    for task in tasks.drain(..) {
        task.abort();
    }
}

pub fn generate_qr_payload(endpoint: String) -> Result<String, ContinueFfiError> {
    let mut lock = CORE.lock().unwrap();
    let state = lock.as_mut().ok_or(ContinueFfiError::NotInitialized)?;

    let mut initiator = InitiatorPairing::new(
        state.identity_signer.clone(),
        state.transport_cert.clone(),
        state.trust_store.clone(),
        state.replay_cache.clone(),
    );

    let qr = initiator
        .generate_qr(endpoint)
        .map_err(|e| ContinueFfiError::PairingFailed(e.to_string()))?;

    Ok(qr.encode())
}

pub fn start_pairing_server(
    listen_port: u16,
    advertised_endpoint: String,
) -> Result<String, ContinueFfiError> {
    let mut lock = CORE.lock().unwrap();
    let state = lock.as_mut().ok_or(ContinueFfiError::NotInitialized)?;

    if let Some(existing) = state.active_pairing.take() {
        existing.server_endpoint.close(0u32.into(), b"superseded");
    }

    let recorded_spki: Arc<Mutex<Option<[u8; 32]>>> = Arc::new(Mutex::new(None));
    let server_tls = state
        .transport_cert
        .build_pairing_server_tls(recorded_spki.clone())
        .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;

    let bind_addr = std::net::SocketAddr::from(([0, 0, 0, 0], listen_port));
    let server_endpoint = transport::create_server_endpoint(bind_addr, server_tls)
        .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;

    let mut initiator = InitiatorPairing::new(
        state.identity_signer.clone(),
        state.transport_cert.clone(),
        state.trust_store.clone(),
        state.replay_cache.clone(),
    );

    let qr = initiator
        .generate_qr(advertised_endpoint)
        .map_err(|e| ContinueFfiError::PairingFailed(e.to_string()))?;

    let (tx, rx) = tokio::sync::oneshot::channel();
    let endpoint_clone = server_endpoint.clone();
    let trust_store = state.trust_store.clone();
    let registry = state.sessions.clone();

    state.runtime.spawn(async move {
        let incoming = match endpoint_clone.accept().await {
            Some(inc) => inc,
            None => {
                let _ = tx.send(Err(transport::TransportError::HandshakeFailed(
                    "Listener closed".to_string(),
                )
                .into()));
                return;
            }
        };

        let conn = match incoming.await {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Err(transport::TransportError::HandshakeFailed(format!(
                    "Connection failed: {e}"
                ))
                .into()));
                return;
            }
        };

        let (mut send_stream, mut recv_stream) = match conn.accept_bi().await {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(Err(transport::TransportError::HandshakeFailed(format!(
                    "Stream accept failed: {e}"
                ))
                .into()));
                return;
            }
        };

        let recorded_hash = match *recorded_spki.lock().unwrap() {
            Some(h) => h,
            None => {
                let _ = tx.send(Err(pairing::PairingError::SpkiMismatch));
                return;
            }
        };

        let result = initiator
            .complete_handshake(&mut send_stream, &mut recv_stream, recorded_hash)
            .await;
        if let Ok(peer) = &result {
            sessions::remember_peer_address(
                &trust_store,
                &peer.fingerprint,
                conn.remote_address().ip(),
            );
            registry.redial_now();
        }
        let _ = tx.send(result);
    });

    state.active_pairing = Some(ActivePairing {
        server_endpoint,
        result_rx: rx,
    });

    Ok(qr.encode())
}

pub fn await_pairing_result(timeout_secs: u32) -> Result<TrustedPeerFfi, ContinueFfiError> {
    let (runtime, trusted_keys, mut active_pairing) = {
        let mut lock = CORE.lock().unwrap();
        let state = lock.as_mut().ok_or(ContinueFfiError::NotInitialized)?;
        let pairing = state.active_pairing.take().ok_or_else(|| {
            ContinueFfiError::InternalError("No active pairing server".to_string())
        })?;
        (state.runtime.clone(), state.trusted_keys.clone(), pairing)
    };

    let result = runtime.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs as u64),
            &mut active_pairing.result_rx,
        )
        .await
    });

    active_pairing
        .server_endpoint
        .close(0u32.into(), b"complete");

    match result {
        Ok(Ok(Ok(peer))) => {
            trust_key(&trusted_keys, peer.transport_spki_hash);
            Ok(peer.into())
        }
        Ok(Ok(Err(pairing_err))) => Err(ContinueFfiError::PairingFailed(pairing_err.to_string())),
        Ok(Err(_channel_closed)) => Err(ContinueFfiError::PairingFailed(
            "Pairing cancelled or aborted".to_string(),
        )),
        Err(_elapsed) => Err(ContinueFfiError::PairingTimeout),
    }
}

pub fn cancel_pairing() -> Result<(), ContinueFfiError> {
    let mut lock = CORE.lock().unwrap();
    let state = lock.as_mut().ok_or(ContinueFfiError::NotInitialized)?;

    if let Some(pairing) = state.active_pairing.take() {
        pairing.server_endpoint.close(0u32.into(), b"cancelled");
    }
    Ok(())
}

pub fn pair_from_qr(qr_payload: String) -> Result<TrustedPeerFfi, ContinueFfiError> {
    let (runtime, transport_cert, identity_signer, trust_store, trusted_keys, registry) = {
        let lock = CORE.lock().unwrap();
        let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
        (
            state.runtime.clone(),
            state.transport_cert.clone(),
            state.identity_signer.clone(),
            state.trust_store.clone(),
            state.trusted_keys.clone(),
            state.sessions.clone(),
        )
    };

    runtime.block_on(async move {
        let qr = pairing::QrPayload::decode(&qr_payload)
            .map_err(|e| ContinueFfiError::InvalidQr(e.to_string()))?;

        let addr: std::net::SocketAddr = qr
            .endpoint
            .parse()
            .map_err(|e| ContinueFfiError::InvalidQr(format!("Invalid endpoint address: {e}")))?;

        let connection = transport::connect_pinned(
            &transport_cert,
            qr.transport_spki_hash,
            addr,
            &DialConfig::default(),
        )
        .await
        .map_err(|e| ContinueFfiError::PairingFailed(format!("Connect failed: {e}")))?;

        let (mut send_stream, mut recv_stream) = connection
            .open_bi()
            .await
            .map_err(|e| ContinueFfiError::PairingFailed(format!("Stream open failed: {e}")))?;

        let responder =
            pairing::ResponderPairing::new(identity_signer, transport_cert, trust_store.clone());
        let trusted_peer = responder
            .complete_handshake(&qr, &mut send_stream, &mut recv_stream)
            .await
            .map_err(|e| ContinueFfiError::PairingFailed(e.to_string()))?;

        trust_key(&trusted_keys, trusted_peer.transport_spki_hash);
        sessions::remember_peer_address(&trust_store, &trusted_peer.fingerprint, addr.ip());
        registry.redial_now();
        Ok(trusted_peer.into())
    })
}

fn trust_key(trusted_keys: &RwLock<HashSet<[u8; 32]>>, key: [u8; 32]) {
    trusted_keys
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key);
}

pub fn list_trusted_peers() -> Result<Vec<TrustedPeerFfi>, ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;

    let peers = state
        .trust_store
        .list_peers()
        .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?;

    Ok(peers.into_iter().map(Into::into).collect())
}

pub fn remove_trusted_peer(fingerprint: String) -> Result<bool, ContinueFfiError> {
    let (runtime, trust_store, sessions, trusted_keys) = {
        let lock = CORE.lock().unwrap();
        let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
        (
            state.runtime.clone(),
            state.trust_store.clone(),
            state.sessions.clone(),
            state.trusted_keys.clone(),
        )
    };

    // Revoke the key before closing the session so the device can't reconnect in between.
    if let Ok(Some(peer)) = trust_store.get_peer(&fingerprint) {
        trusted_keys
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&peer.transport_spki_hash);
    }
    let removed = trust_store
        .remove_peer(&fingerprint)
        .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?;
    runtime.block_on(async { sessions.remove(&fingerprint).await });
    Ok(removed)
}

pub fn get_capabilities() -> Result<Vec<u32>, ContinueFfiError> {
    Ok(vec![
        CapabilityId::FILE_TRANSFER.raw(),
        CapabilityId::CLIPBOARD.raw(),
        CapabilityId::NOTIFICATIONS.raw(),
    ])
}

pub fn query_permission(
    peer_fingerprint: String,
    capability_id: u32,
) -> Result<String, ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;

    let status = state
        .permission_store
        .query_state(&peer_fingerprint, CapabilityId(capability_id))
        .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?;

    Ok(match status {
        permissions::PermissionState::Allow => "Allow".to_string(),
        permissions::PermissionState::Deny => "Deny".to_string(),
        permissions::PermissionState::Ask => "Ask".to_string(),
        permissions::PermissionState::AllowOnce => "AllowOnce".to_string(),
    })
}

pub fn set_permission(
    peer_fingerprint: String,
    capability_id: u32,
    grant: String,
) -> Result<(), ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;

    let parsed_grant = match grant.as_str() {
        "Allow" => PersistedGrant::Allow,
        "Deny" => PersistedGrant::Deny,
        "Ask" => PersistedGrant::Ask,
        "AllowOnce" => {
            state
                .permission_store
                .grant_allow_once(&peer_fingerprint, CapabilityId(capability_id));
            return Ok(());
        }
        _ => {
            return Err(ContinueFfiError::InternalError(format!(
                "Invalid grant string: {grant}"
            )))
        }
    };

    state
        .permission_store
        .set_persisted_grant(
            &peer_fingerprint,
            CapabilityId(capability_id),
            1,
            parsed_grant,
        )
        .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?;

    Ok(())
}

pub fn revoke_permission(
    peer_fingerprint: String,
    capability_id: u32,
) -> Result<(), ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;

    state
        .permission_store
        .clear_allow_once_for_peer(&peer_fingerprint);

    state
        .permission_store
        .set_persisted_grant(
            &peer_fingerprint,
            CapabilityId(capability_id),
            1,
            PersistedGrant::Deny,
        )
        .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?;

    Ok(())
}

pub fn connect_to_peer(peer_fingerprint: String, endpoint: String) -> Result<(), ContinueFfiError> {
    let (runtime, trust_store, sessions) = {
        let lock = CORE.lock().unwrap();
        let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
        (
            state.runtime.clone(),
            state.trust_store.clone(),
            state.sessions.clone(),
        )
    };

    let peer = trust_store
        .get_peer(&peer_fingerprint)
        .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?
        .ok_or_else(|| {
            ContinueFfiError::InternalError("Peer not found in trust store".to_string())
        })?;

    let addr: std::net::SocketAddr = endpoint
        .parse()
        .map_err(|e| ContinueFfiError::InternalError(format!("Invalid endpoint address: {e}")))?;

    runtime.block_on(async move {
        sessions
            .connect(&peer_fingerprint, peer.transport_spki_hash, addr)
            .await
            .map_err(|e| ContinueFfiError::InternalError(format!("Connect failed: {e}")))
    })
}

pub fn is_peer_connected(peer_fingerprint: String) -> Result<bool, ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
    Ok(state.sessions.state(&peer_fingerprint) == SessionState::Connected)
}

pub fn reconnect(peer_fingerprint: String) -> Result<(), ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
    state.sessions.resume_auto_connect(&peer_fingerprint);
    Ok(())
}

pub fn disconnect(peer_fingerprint: String) -> Result<(), ContinueFfiError> {
    let (runtime, sessions) = {
        let lock = CORE.lock().unwrap();
        let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
        (state.runtime.clone(), state.sessions.clone())
    };

    runtime.block_on(async move { sessions.disconnect(&peer_fingerprint).await });
    Ok(())
}

struct Connected {
    runtime: Arc<tokio::runtime::Runtime>,
    mux: Arc<SessionMultiplexer>,
    history: HistoryStore,
    peer_name: String,
}

fn connected_session(peer_fingerprint: &str) -> Result<Connected, ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
    let mux = state
        .sessions
        .get(peer_fingerprint)
        .ok_or_else(|| ContinueFfiError::InternalError("Peer not connected".to_string()))?;
    Ok(Connected {
        runtime: state.runtime.clone(),
        mux,
        history: state.history.clone(),
        peer_name: peer_name(&state.trust_store, peer_fingerprint),
    })
}

/// The file is the app's temporary copy, so history keeps its name but not its place.
pub fn send_file(peer_fingerprint: String, file_path: String) -> Result<u64, ContinueFfiError> {
    let Connected {
        runtime,
        mux,
        history,
        peer_name,
    } = connected_session(&peer_fingerprint)?;
    let path = std::path::Path::new(&file_path);
    let result = runtime.block_on(async {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let transfer_id = format!("tx-{now}");
        mux.send_file_to_peer(path, transfer_id, None::<fn(u64, u64)>)
            .await
    });
    remember(
        &history,
        history::Item {
            direction: Direction::Sent,
            kind: Kind::File,
            label: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            peer_fingerprint,
            peer_name,
            size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
            failed: result.is_err(),
            location: None,
        },
    );
    result.map_err(|e| ContinueFfiError::InternalError(e.to_string()))
}

pub fn send_clipboard_text(peer_fingerprint: String, text: String) -> Result<(), ContinueFfiError> {
    let Connected {
        runtime,
        mux,
        history,
        peer_name,
    } = connected_session(&peer_fingerprint)?;
    let sent = text.clone();
    let result = runtime.block_on(async move {
        let mut caps = std::collections::HashSet::new();
        caps.insert(protocol::CapabilityId::CLIPBOARD);
        let query = capabilities::CapabilityQuery {
            capability: protocol::CapabilityId::CLIPBOARD,
            is_os_available: true,
            is_app_permitted: true,
            is_peer_authorized: true,
            negotiated_session_capabilities: caps,
        };

        mux.send_clipboard_to_peer(
            clipboard::ClipboardFormat::TextPlain,
            sent.into_bytes(),
            &query,
        )
        .await
    });
    remember(
        &history,
        history::Item {
            direction: Direction::Sent,
            kind: Kind::Text,
            size: text.len() as u64,
            label: text,
            peer_fingerprint,
            peer_name,
            failed: result.is_err(),
            location: None,
        },
    );
    result
        .map(|_| ())
        .map_err(|e| ContinueFfiError::InternalError(e.to_string()))
}

pub fn send_notification(
    peer_fingerprint: String,
    title: String,
    body: String,
    app_name: String,
) -> Result<(), ContinueFfiError> {
    let Connected { runtime, mux, .. } = connected_session(&peer_fingerprint)?;

    runtime.block_on(async move {
        let dispatcher = notifications::NotificationDispatcher::new();
        let mut caps = std::collections::HashSet::new();
        caps.insert(protocol::CapabilityId::NOTIFICATIONS);
        let query = capabilities::CapabilityQuery {
            capability: protocol::CapabilityId::NOTIFICATIONS,
            is_os_available: true,
            is_app_permitted: true,
            is_peer_authorized: true,
            negotiated_session_capabilities: caps,
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let post = notifications::NotificationPost {
            notification_id: format!("notif-{now}"),
            package_name: "continue.ffi".to_string(),
            app_name,
            title,
            body,
            timestamp: now,
            actions: vec![],
        };

        mux.send_notification_to_peer(&dispatcher, post, &query)
            .await
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;

        Ok(())
    })
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    let mut s = String::new();
    for b in bytes.as_ref() {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stands in for the Android Keystore.
    #[derive(Default)]
    struct MemoryStore(Mutex<std::collections::HashMap<String, Vec<u8>>>);

    impl SecretStoreFfi for Arc<MemoryStore> {
        fn store(&self, label: String, secret: Vec<u8>) -> Result<(), SecretStoreFfiError> {
            self.0.lock().unwrap().insert(label, secret);
            Ok(())
        }
        fn load(&self, label: String) -> Result<Option<Vec<u8>>, SecretStoreFfiError> {
            Ok(self.0.lock().unwrap().get(&label).cloned())
        }
        fn delete(&self, label: String) -> Result<(), SecretStoreFfiError> {
            self.0.lock().unwrap().remove(&label);
            Ok(())
        }
    }

    // One test, since the key store is shared by the whole process.
    #[test]
    fn keys_move_from_files_into_the_apps_store() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("continue-ffi-keys-{nanos}"));
        let db = dir.join("continue.db").to_string_lossy().into_owned();
        std::fs::create_dir_all(&dir).unwrap();

        // An earlier version, with no store set.
        let before = load_device_keys(&db).unwrap();
        let key = |keys: &DeviceKeys| keys.identity_signer.verifying_key().unwrap();

        let app_store = Arc::new(MemoryStore::default());
        set_key_store(Box::new(app_store.clone()));
        let after = load_device_keys(&db).unwrap();
        *KEY_STORE.lock().unwrap() = None;

        assert_eq!(key(&after), key(&before));
        assert_eq!(
            after.transport_cert.spki_hash,
            before.transport_cert.spki_hash
        );
        let saved = app_store.0.lock().unwrap();
        assert!(saved.contains_key("device_identity") && saved.contains_key("transport_cert"));
        let files = std::fs::read_dir(dir.join("secrets"))
            .unwrap()
            .filter(|entry| entry.as_ref().unwrap().path().extension() == Some("secret".as_ref()))
            .count();
        assert_eq!(files, 0, "no key left in files");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // One test, since the question queue is shared by the whole process.
    #[tokio::test(flavor = "multi_thread")]
    async fn questions_reach_the_app_and_answers_reach_the_core() {
        let none = tokio::task::spawn_blocking(|| next_permission_request(10))
            .await
            .unwrap();
        assert!(none.is_none());

        let prompt = permission_prompt(TrustStore::in_memory().unwrap());
        let decision = prompt(sessions::PermissionRequest {
            peer: "phone".to_string(),
            capability: CapabilityId::FILE_TRANSFER,
            detail: Some("photo.jpg".to_string()),
            deadline: tokio::time::Instant::now() + sessions::PROMPT_TIMEOUT,
        });

        let question = tokio::task::spawn_blocking(|| next_permission_request(1000))
            .await
            .unwrap()
            .expect("a question");
        assert_eq!(question.peer_fingerprint, "phone");
        assert_eq!(question.capability_id, CapabilityId::FILE_TRANSFER.raw());
        assert_eq!(question.detail.as_deref(), Some("photo.jpg"));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        assert!(question.expires_at > now && question.expires_at <= now + 30_000);

        answer_permission_request(question.id, PermissionDecisionFfi::AlwaysAllow);
        assert_eq!(decision.await.unwrap(), PermissionDecision::AlwaysAllow);
    }

    #[test]
    fn received_files_and_text_reach_the_app_with_their_sender() {
        let history = HistoryStore::in_memory().unwrap();
        let handlers = deliver_received(
            sessions::SessionCapabilityHandlers::new(std::env::temp_dir()),
            TrustStore::in_memory().unwrap(),
            history.clone(),
        );
        (handlers.on_file_received.unwrap())(
            "laptop",
            transfer::ReceivedFile {
                transfer_id: "tx-1".to_string(),
                path: "/data/received/report.pdf".into(),
                file_name: "report.pdf".to_string(),
                bytes_received: 2048,
            },
        );

        let file = next_received(100).expect("the file");
        assert_eq!(file.peer_fingerprint, "laptop");
        assert_eq!(file.file_name.as_deref(), Some("report.pdf"));
        assert_eq!(file.size, 2048);
        assert!(file.text.is_none());
        assert!(next_received(10).is_none());

        let saved = history.list(10).unwrap();
        assert_eq!(file.history_id, Some(saved[0].id));
        assert_eq!(saved[0].item.direction, Direction::Received);
        assert_eq!(saved[0].item.label, "report.pdf");
    }
}
