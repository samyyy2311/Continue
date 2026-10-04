// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

// Generated UniFFI scaffolding (uniffi 0.28) leaves blank lines after doc comments.
#![allow(clippy::empty_line_after_doc_comments)]

uniffi::include_scaffolding!("continue");

use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use thiserror::Error;

use device::{Device, PairError, PairingServer, Stores};
use history::{Direction, Kind};
use identity::FileSecretStore;
use pairing::{DeviceKeys, TrustedPeer};
use permissions::{PermissionState, PersistedGrant};
use protocol::CapabilityId;
use sessions::{PermissionDecision, SessionState};
use tokio::runtime::Runtime;
use transport::TransportCertificate;

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

fn internal(error: impl ToString) -> ContinueFfiError {
    ContinueFfiError::InternalError(error.to_string())
}

fn database(error: impl ToString) -> ContinueFfiError {
    ContinueFfiError::DatabaseError(error.to_string())
}

impl From<device::DeviceError> for ContinueFfiError {
    fn from(error: device::DeviceError) -> Self {
        match error {
            device::DeviceError::Signer(_) => internal(error),
            _ => database(error),
        }
    }
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

struct CoreState {
    runtime: Arc<Runtime>,
    device: Device,
    active_pairing: Option<PairingServer>,
    listener: quinn::Endpoint,
    discovery_tasks: Vec<tokio::task::JoinHandle<()>>,
    incoming: sessions::IncomingFiles,
    /// The name paired devices see for this phone.
    this_device: sessions::ThisDevice,
}

static CORE: Mutex<Option<CoreState>> = Mutex::new(None);

fn with_core<T>(
    f: impl FnOnce(&mut CoreState) -> Result<T, ContinueFfiError>,
) -> Result<T, ContinueFfiError> {
    f(CORE
        .lock()
        .unwrap()
        .as_mut()
        .ok_or(ContinueFfiError::NotInitialized)?)
}

/// The device and the runtime it runs on, for calls that wait without holding up the rest.
fn device() -> Result<(Arc<Runtime>, Device), ContinueFfiError> {
    with_core(|core| Ok((core.runtime.clone(), core.device.clone())))
}

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

/// Saves received files and text to history and hands them to the app through
/// `next_received`.
fn deliver_received(
    mut handlers: sessions::SessionCapabilityHandlers,
    stores: Stores,
) -> sessions::SessionCapabilityHandlers {
    let names = stores.clone();
    handlers.on_device_info = Some(Arc::new(move |peer, device| {
        names.rename_peer(peer, &device.name);
    }));
    let files = stores.clone();
    handlers.on_file_received = Some(Arc::new(move |peer, file| {
        // The app moves the file on, then sets its location.
        let saved = files.received_file(peer, &file, None);
        RECEIVED.push(
            ReceivedFfi {
                history_id: saved.history_id,
                peer_fingerprint: peer.to_string(),
                peer_name: saved.peer_name,
                file_path: Some(file.path.to_string_lossy().into_owned()),
                file_name: Some(file.file_name),
                size: file.bytes_received,
                text: None,
            },
            RECEIVED_LIMIT,
        );
    }));
    handlers.on_clipboard_received = Some(Arc::new(move |peer, update| {
        let text = String::from_utf8_lossy(&update.payload).into_owned();
        let saved = stores.received_text(peer, &text);
        RECEIVED.push(
            ReceivedFfi {
                history_id: saved.history_id,
                peer_fingerprint: peer.to_string(),
                peer_name: saved.peer_name,
                file_path: None,
                file_name: None,
                size: update.payload.len() as u64,
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

fn history() -> Result<history::HistoryStore, ContinueFfiError> {
    with_core(|core| Ok(core.device.stores.history.clone()))
}

/// What this phone sent and received, newest first.
pub fn list_history(limit: u32) -> Result<Vec<HistoryEntryFfi>, ContinueFfiError> {
    let entries = history()?.list(limit).map_err(database)?;
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
    history()?.clear().map_err(database)
}

/// Notes where the app put a received file.
pub fn set_history_location(id: i64, location: String) -> Result<(), ContinueFfiError> {
    history()?.set_location(id, &location).map_err(database)
}

/// Waits up to `timeout_ms` for the next file or text a paired device sent.
pub fn next_received(timeout_ms: u32) -> Option<ReceivedFfi> {
    RECEIVED.next(Duration::from_millis(timeout_ms.into()))
}

fn permission_prompt(stores: Stores) -> sessions::PermissionPrompt {
    Arc::new(move |request| {
        let (answer, decision) = tokio::sync::oneshot::channel();
        let id = QUESTIONS.next_id.fetch_add(1, Ordering::Relaxed);
        QUESTIONS.answers.lock().unwrap().insert(id, answer);
        QUESTIONS.waiting.push(
            PermissionRequestFfi {
                id,
                peer_name: stores.peer_name(&request.peer),
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
            .map_err(internal)?,
    );
    let stores = Stores::open(&db_path)?;
    let keys = load_device_keys(&db_path)?;

    // Next to the database, in the app's own storage; the app moves files on from there.
    let download_dir = Path::new(&db_path)
        .parent()
        .unwrap_or(Path::new("."))
        .join("received");
    let _ = std::fs::create_dir_all(&download_dir);
    let incoming = sessions::IncomingFiles::default();
    // Named by `set_device_name` once the app has read the phone's name.
    let this_device = sessions::ThisDevice::default();
    let handlers = deliver_received(
        sessions::SessionCapabilityHandlers::new(download_dir)
            .with_this_device(this_device.clone())
            .with_incoming(incoming.clone())
            .with_permission_prompt(permission_prompt(stores.clone())),
        stores.clone(),
    );
    let device = Device::new(stores, keys, handlers, None)?;
    let listener = {
        let _runtime = runtime.enter();
        device
            .listen()
            .map_err(|e| internal(format!("Could not listen: {e}")))?
    };

    *CORE.lock().unwrap() = Some(CoreState {
        runtime,
        device,
        active_pairing: None,
        listener,
        discovery_tasks: Vec::new(),
        incoming,
        this_device,
    });
    Ok(())
}

/// Keys live in the app's secret store (the Android Keystore) so pairings survive a restart.
/// Keys from earlier versions, in a `secrets` folder beside the database, move there; without
/// a store they stay in that folder. An in-memory database (used by tests) gets throwaway keys.
fn load_device_keys(db_path: &str) -> Result<DeviceKeys, ContinueFfiError> {
    if db_path == ":memory:" {
        let seed = crypto::keys::generate_ed25519_seed();
        let signing_key = crypto::keys::signing_key_from_seed(&seed.0);
        return Ok(DeviceKeys {
            identity_signer: Arc::new(identity::InMemorySigner::new(signing_key)),
            transport_cert: Arc::new(TransportCertificate::generate().map_err(internal)?),
        });
    }

    let secrets_dir = Path::new(db_path)
        .parent()
        .ok_or_else(|| internal(format!("Database path has no folder: {db_path}")))?
        .join("secrets");
    let files = FileSecretStore::new(secrets_dir).map_err(internal)?;
    let app_store = KEY_STORE.lock().unwrap().clone();
    match app_store {
        Some(app_store) => DeviceKeys::load_or_create(&identity::PlatformFirstStore::new(
            Box::new(AppSecretStore(app_store)),
            files,
        )),
        None => DeviceKeys::load_or_create(&files),
    }
    .map_err(internal)
}

/// Sets the name paired devices see for this phone, from the next session on.
pub fn set_device_name(name: String) -> Result<(), ContinueFfiError> {
    with_core(|core| {
        core.this_device.set_name(&name);
        Ok(())
    })
}

pub fn get_device_fingerprint() -> Result<String, ContinueFfiError> {
    with_core(|core| Ok(core.device.fingerprint.clone()))
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
    with_core(|core| {
        Ok(core
            .incoming
            .list()
            .into_iter()
            .map(|file| IncomingFileFfi {
                peer_name: core.device.stores.peer_name(&file.peer),
                transfer_id: file.transfer_id,
                file_name: file.file_name,
                received: file.received,
                total: file.total,
            })
            .collect())
    })
    .unwrap_or_default()
}

/// Stops a file part way. False if it already finished or never started.
pub fn cancel_incoming(transfer_id: String) -> bool {
    with_core(|core| Ok(core.incoming.cancel(&transfer_id))).unwrap_or(false)
}

pub fn get_device_spki_hash() -> Result<String, ContinueFfiError> {
    with_core(|core| Ok(hex::encode(core.device.keys.transport_cert.spki_hash)))
}

/// Advertises this device's listener and connects to paired devices as they appear on
/// the network, until `stop_discovery`.
pub fn start_discovery(protocol_version: u32) -> Result<(), ContinueFfiError> {
    with_core(|core| {
        let port = core.listener.local_addr().map_err(internal)?.port();
        stop_tasks(&mut core.discovery_tasks);
        let _runtime = core.runtime.enter();
        core.discovery_tasks = core.device.discover(port, protocol_version).into();
        Ok(())
    })
}

pub fn stop_discovery() -> Result<(), ContinueFfiError> {
    with_core(|core| {
        stop_tasks(&mut core.discovery_tasks);
        Ok(())
    })
}

fn stop_tasks(tasks: &mut Vec<tokio::task::JoinHandle<()>>) {
    for task in tasks.drain(..) {
        task.abort();
    }
}

pub fn generate_qr_payload(endpoint: String) -> Result<String, ContinueFfiError> {
    with_core(|core| {
        let qr = core.device.initiator().generate_qr(endpoint);
        Ok(qr.map_err(pairing_failed)?.encode())
    })
}

fn pairing_failed(error: impl ToString) -> ContinueFfiError {
    ContinueFfiError::PairingFailed(error.to_string())
}

pub fn start_pairing_server(
    listen_port: u16,
    advertised_endpoint: String,
) -> Result<String, ContinueFfiError> {
    with_core(|core| {
        // Dropping the one before stops it.
        core.active_pairing = None;
        let _runtime = core.runtime.enter();
        let server = core
            .device
            .start_pairing(listen_port, |_| advertised_endpoint)
            .map_err(internal)?;
        let code = server.code.clone();
        core.active_pairing = Some(server);
        Ok(code)
    })
}

pub fn await_pairing_result(timeout_secs: u32) -> Result<TrustedPeerFfi, ContinueFfiError> {
    let (runtime, server) = with_core(|core| {
        let server = core
            .active_pairing
            .take()
            .ok_or_else(|| internal("No active pairing server"))?;
        Ok((core.runtime.clone(), server))
    })?;
    let timeout = Duration::from_secs(timeout_secs.into());
    match runtime.block_on(tokio::time::timeout(timeout, server.finish())) {
        Ok(result) => Ok(result.map_err(pairing_failed)?.into()),
        Err(_elapsed) => Err(ContinueFfiError::PairingTimeout),
    }
}

pub fn cancel_pairing() -> Result<(), ContinueFfiError> {
    with_core(|core| {
        core.active_pairing = None;
        Ok(())
    })
}

pub fn pair_from_qr(qr_payload: String) -> Result<TrustedPeerFfi, ContinueFfiError> {
    let (runtime, device) = device()?;
    match runtime.block_on(device.pair_with_code(&qr_payload)) {
        Ok(peer) => Ok(peer.into()),
        Err(PairError::BadCode(reason)) => Err(ContinueFfiError::InvalidQr(reason)),
        Err(error) => Err(pairing_failed(error)),
    }
}

pub fn list_trusted_peers() -> Result<Vec<TrustedPeerFfi>, ContinueFfiError> {
    let peers = with_core(|core| core.device.stores.trust.list_peers().map_err(database))?;
    Ok(peers.into_iter().map(Into::into).collect())
}

pub fn remove_trusted_peer(fingerprint: String) -> Result<bool, ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime
        .block_on(device.forget(&fingerprint))
        .map_err(database)
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
    with_core(|core| {
        let state = core
            .device
            .stores
            .permissions
            .query_state(&peer_fingerprint, CapabilityId(capability_id))
            .map_err(database)?;
        Ok(state.as_str().to_string())
    })
}

pub fn set_permission(
    peer_fingerprint: String,
    capability_id: u32,
    grant: String,
) -> Result<(), ContinueFfiError> {
    let state = PermissionState::parse(&grant)
        .ok_or_else(|| internal(format!("Invalid grant string: {grant}")))?;
    with_core(|core| {
        core.device
            .stores
            .permissions
            .set_state(&peer_fingerprint, CapabilityId(capability_id), state)
            .map_err(database)
    })
}

pub fn revoke_permission(
    peer_fingerprint: String,
    capability_id: u32,
) -> Result<(), ContinueFfiError> {
    with_core(|core| {
        let permissions = &core.device.stores.permissions;
        permissions.clear_allow_once_for_peer(&peer_fingerprint);
        permissions
            .set_persisted_grant(
                &peer_fingerprint,
                CapabilityId(capability_id),
                1,
                PersistedGrant::Deny,
            )
            .map_err(database)
    })
}

pub fn connect_to_peer(peer_fingerprint: String, endpoint: String) -> Result<(), ContinueFfiError> {
    let addr: std::net::SocketAddr = endpoint
        .parse()
        .map_err(|e| internal(format!("Invalid endpoint address: {e}")))?;
    let (runtime, device) = device()?;
    match runtime.block_on(device.connect(&peer_fingerprint, addr)) {
        Ok(()) => Ok(()),
        Err(device::ConnectError::Store(error)) => Err(database(error)),
        Err(error) => Err(internal(error)),
    }
}

pub fn is_peer_connected(peer_fingerprint: String) -> Result<bool, ContinueFfiError> {
    with_core(|core| Ok(core.device.sessions.state(&peer_fingerprint) == SessionState::Connected))
}

pub fn reconnect(peer_fingerprint: String) -> Result<(), ContinueFfiError> {
    with_core(|core| {
        core.device.sessions.resume_auto_connect(&peer_fingerprint);
        Ok(())
    })
}

pub fn disconnect(peer_fingerprint: String) -> Result<(), ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime.block_on(device.sessions.disconnect(&peer_fingerprint));
    Ok(())
}

/// The file is the app's temporary copy, so history keeps its name but not its place. If the
/// connection drops part way, this waits for it to come back and sends the rest.
pub fn send_file(peer_fingerprint: String, file_path: String) -> Result<u64, ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime
        .block_on(device.send_file(
            &peer_fingerprint,
            Path::new(&file_path),
            None,
            None::<fn(u64, u64)>,
        ))
        .map_err(internal)
}

pub fn send_clipboard_text(peer_fingerprint: String, text: String) -> Result<(), ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime
        .block_on(device.send_text(&peer_fingerprint, text))
        .map_err(internal)
}

pub fn send_notification(
    peer_fingerprint: String,
    title: String,
    body: String,
    app_name: String,
) -> Result<(), ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime
        .block_on(device.send_notification(
            &peer_fingerprint,
            "continue.ffi",
            app_name,
            title,
            body,
        ))
        .map_err(internal)
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

        let prompt = permission_prompt(memory_stores());
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

    fn memory_stores() -> Stores {
        Stores {
            trust: pairing::TrustStore::in_memory().unwrap(),
            permissions: Arc::new(permissions::PermissionStore::in_memory().unwrap()),
            history: history::HistoryStore::in_memory().unwrap(),
        }
    }

    #[test]
    fn received_files_and_text_reach_the_app_with_their_sender() {
        let stores = memory_stores();
        let history = stores.history.clone();
        let handlers = deliver_received(
            sessions::SessionCapabilityHandlers::new(std::env::temp_dir()),
            stores,
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
