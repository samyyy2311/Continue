// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

// Generated UniFFI scaffolding (uniffi 0.28) leaves blank lines after doc comments.
#![allow(clippy::empty_line_after_doc_comments)]

uniffi::include_scaffolding!("continue");

use std::collections::HashSet;
use std::sync::{Arc, Mutex, RwLock};
use thiserror::Error;

use identity::{FileSecretStore, IdentitySigner};
use pairing::{DeviceKeys, InitiatorPairing, ReplayCache, TrustStore, TrustedPeer};
use permissions::{PermissionStore, PersistedGrant};
use protocol::CapabilityId;
use sessions::{SessionMultiplexer, SessionRegistry, SessionState};
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

struct ActivePairing {
    server_endpoint: quinn::Endpoint,
    result_rx: tokio::sync::oneshot::Receiver<Result<TrustedPeer, pairing::PairingError>>,
}

struct CoreState {
    runtime: Arc<tokio::runtime::Runtime>,
    trust_store: TrustStore,
    permission_store: PermissionStore,
    transport_cert: Arc<TransportCertificate>,
    identity_signer: Arc<dyn IdentitySigner>,
    replay_cache: Arc<ReplayCache>,
    active_pairing: Option<ActivePairing>,
    sessions: SessionRegistry,
    /// Transport keys of paired devices; only these may connect to `listener`.
    trusted_keys: Arc<RwLock<HashSet<[u8; 32]>>>,
    listener: quinn::Endpoint,
    discovery_tasks: Vec<tokio::task::JoinHandle<()>>,
}

static CORE: Mutex<Option<CoreState>> = Mutex::new(None);

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
    if let Some(previous) = previous {
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
    let download_dir = std::env::temp_dir().join("continue_downloads");
    let _ = std::fs::create_dir_all(&download_dir);
    let grants = permission_store.clone();
    let sessions = SessionRegistry::new(
        local_fingerprint,
        transport_cert.clone(),
        sessions::SessionCapabilityHandlers::new(download_dir),
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
        transport_cert,
        identity_signer,
        replay_cache: Arc::new(ReplayCache::new()),
        active_pairing: None,
        sessions,
        trusted_keys,
        listener,
        discovery_tasks: Vec::new(),
    };

    let mut lock = CORE.lock().unwrap();
    *lock = Some(state);
    Ok(())
}

/// Keys live in a `secrets` folder beside the database so pairings survive a
/// restart. An in-memory database (used by tests) gets throwaway keys.
// The files sit in app-private storage. Wrapping them with the Android Keystore comes later.
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
    let store = FileSecretStore::new(secrets_dir).map_err(|e| internal(e.to_string()))?;
    DeviceKeys::load_or_create(&store).map_err(|e| internal(e.to_string()))
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

fn connected_session(
    peer_fingerprint: &str,
) -> Result<(Arc<tokio::runtime::Runtime>, Arc<SessionMultiplexer>), ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
    let mux = state
        .sessions
        .get(peer_fingerprint)
        .ok_or_else(|| ContinueFfiError::InternalError("Peer not connected".to_string()))?;
    Ok((state.runtime.clone(), mux))
}

pub fn send_file(peer_fingerprint: String, file_path: String) -> Result<u64, ContinueFfiError> {
    let (runtime, mux) = connected_session(&peer_fingerprint)?;

    runtime.block_on(async move {
        let path = std::path::Path::new(&file_path);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let transfer_id = format!("tx-{now}");
        mux.send_file_to_peer(path, transfer_id, None::<fn(u64, u64)>)
            .await
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))
    })
}

pub fn send_clipboard_text(peer_fingerprint: String, text: String) -> Result<(), ContinueFfiError> {
    let (runtime, mux) = connected_session(&peer_fingerprint)?;

    runtime.block_on(async move {
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
            text.into_bytes(),
            &query,
        )
        .await
        .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;

        Ok(())
    })
}

pub fn send_notification(
    peer_fingerprint: String,
    title: String,
    body: String,
    app_name: String,
) -> Result<(), ContinueFfiError> {
    let (runtime, mux) = connected_session(&peer_fingerprint)?;

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
