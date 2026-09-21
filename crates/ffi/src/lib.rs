// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

uniffi::include_scaffolding!("continue");

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

use discovery::{DiscoveryAdvertiser, EphemeralDiscoveryId};
use identity::IdentitySigner;
use pairing::{InitiatorPairing, ReplayCache, TrustStore, TrustedPeer};
use permissions::{PermissionStore, PersistedGrant};
use protocol::CapabilityId;
use sessions::SessionMultiplexer;
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
    advertiser: Option<DiscoveryAdvertiser>,
    active_pairing: Option<ActivePairing>,
    active_sessions: Arc<Mutex<HashMap<String, Arc<SessionMultiplexer>>>>,
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
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?,
    );

    let trust_store = TrustStore::open(&db_path)
        .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?;
    let permission_store = PermissionStore::open(&db_path)
        .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?;

    let transport_cert = Arc::new(
        TransportCertificate::generate()
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?,
    );

    let seed = crypto::keys::generate_ed25519_seed();
    let signing_key = crypto::keys::signing_key_from_seed(&seed.0);
    let identity_signer: Arc<dyn IdentitySigner> =
        Arc::new(identity::InMemorySigner::new(signing_key));

    let state = CoreState {
        runtime,
        trust_store,
        permission_store,
        transport_cert,
        identity_signer,
        replay_cache: Arc::new(ReplayCache::new()),
        advertiser: None,
        active_pairing: None,
        active_sessions: Arc::new(Mutex::new(HashMap::new())),
    };

    let mut lock = CORE.lock().unwrap();
    *lock = Some(state);
    Ok(())
}

pub fn get_device_fingerprint() -> Result<String, ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
    let key = state
        .identity_signer
        .verifying_key()
        .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;
    Ok(identity::Fingerprint::from_verifying_key(&key).as_str().to_string())
}

pub fn get_device_spki_hash() -> Result<String, ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
    Ok(hex_encode(state.transport_cert.spki_hash))
}

pub fn start_discovery(port: u16, protocol_version: u32) -> Result<(), ContinueFfiError> {
    let mut lock = CORE.lock().unwrap();
    let state = lock.as_mut().ok_or(ContinueFfiError::NotInitialized)?;

    let ephemeral_id = EphemeralDiscoveryId::generate();
    let advertiser = DiscoveryAdvertiser::start(port, ephemeral_id, protocol_version)
        .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;

    state.advertiser = Some(advertiser);
    Ok(())
}

pub fn stop_discovery() -> Result<(), ContinueFfiError> {
    let mut lock = CORE.lock().unwrap();
    let state = lock.as_mut().ok_or(ContinueFfiError::NotInitialized)?;

    if let Some(adv) = state.advertiser.take() {
        adv.unregister()
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;
    }
    Ok(())
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

    state.runtime.spawn(async move {
        let incoming = match endpoint_clone.accept().await {
            Some(inc) => inc,
            None => {
                let _ = tx.send(Err(pairing::PairingError::HandshakeFailed(
                    "Listener closed".to_string(),
                )));
                return;
            }
        };

        let conn = match incoming.await {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Err(pairing::PairingError::HandshakeFailed(format!(
                    "Connection failed: {e}"
                ))));
                return;
            }
        };

        let (mut send_stream, mut recv_stream) = match conn.accept_bi().await {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(Err(pairing::PairingError::HandshakeFailed(format!(
                    "Stream accept failed: {e}"
                ))));
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
        let _ = tx.send(result);
    });

    state.active_pairing = Some(ActivePairing {
        server_endpoint,
        result_rx: rx,
    });

    Ok(qr.encode())
}

pub fn await_pairing_result(timeout_secs: u32) -> Result<TrustedPeerFfi, ContinueFfiError> {
    let (runtime, mut active_pairing) = {
        let mut lock = CORE.lock().unwrap();
        let state = lock.as_mut().ok_or(ContinueFfiError::NotInitialized)?;
        let pairing = state
            .active_pairing
            .take()
            .ok_or_else(|| ContinueFfiError::InternalError("No active pairing server".to_string()))?;
        (state.runtime.clone(), pairing)
    };

    let result = runtime.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs as u64),
            &mut active_pairing.result_rx,
        )
        .await
    });

    active_pairing.server_endpoint.close(0u32.into(), b"complete");

    match result {
        Ok(Ok(Ok(peer))) => Ok(peer.into()),
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
    let (runtime, transport_cert, identity_signer, trust_store) = {
        let lock = CORE.lock().unwrap();
        let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
        (
            state.runtime.clone(),
            state.transport_cert.clone(),
            state.identity_signer.clone(),
            state.trust_store.clone(),
        )
    };

    runtime.block_on(async move {
        let qr = pairing::QrPayload::decode(&qr_payload)
            .map_err(|e| ContinueFfiError::InvalidQr(e.to_string()))?;

        let addr: std::net::SocketAddr = qr.endpoint.parse().map_err(|e| {
            ContinueFfiError::InvalidQr(format!("Invalid endpoint address: {e}"))
        })?;

        let client_tls = transport_cert
            .build_pinned_client_tls(qr.transport_spki_hash)
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;

        let bind_addr: std::net::SocketAddr = if addr.is_ipv6() {
            "[::]:0".parse().unwrap()
        } else {
            "0.0.0.0:0".parse().unwrap()
        };

        let client_endpoint = transport::create_client_endpoint(bind_addr, client_tls)
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;

        let connection = client_endpoint
            .connect(addr, "continue-device")
            .map_err(|e| ContinueFfiError::PairingFailed(format!("Connect failed: {e}")))?
            .await
            .map_err(|e| ContinueFfiError::PairingFailed(format!("Handshake failed: {e}")))?;

        let (mut send_stream, mut recv_stream) = connection
            .open_bi()
            .await
            .map_err(|e| ContinueFfiError::PairingFailed(format!("Stream open failed: {e}")))?;

        let responder = pairing::ResponderPairing::new(identity_signer, transport_cert, trust_store);
        let trusted_peer = responder
            .complete_handshake(&qr, &mut send_stream, &mut recv_stream)
            .await
            .map_err(|e| ContinueFfiError::PairingFailed(e.to_string()))?;

        Ok(trusted_peer.into())
    })
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
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;

    state
        .trust_store
        .remove_peer(&fingerprint)
        .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))
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
        .set_persisted_grant(&peer_fingerprint, CapabilityId(capability_id), 1, parsed_grant)
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
    let (runtime, transport_cert, trust_store, active_sessions) = {
        let lock = CORE.lock().unwrap();
        let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
        (
            state.runtime.clone(),
            state.transport_cert.clone(),
            state.trust_store.clone(),
            state.active_sessions.clone(),
        )
    };

    runtime.block_on(async move {
        let peer = trust_store
            .get_peer(&peer_fingerprint)
            .map_err(|e| ContinueFfiError::DatabaseError(e.to_string()))?
            .ok_or_else(|| ContinueFfiError::InternalError("Peer not found in trust store".to_string()))?;

        let addr: std::net::SocketAddr = endpoint.parse().map_err(|e| {
            ContinueFfiError::InternalError(format!("Invalid endpoint address: {e}"))
        })?;

        let client_tls = transport_cert
            .build_pinned_client_tls(peer.transport_spki_hash)
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;

        let bind_addr: std::net::SocketAddr = if addr.is_ipv6() {
            "[::]:0".parse().unwrap()
        } else {
            "0.0.0.0:0".parse().unwrap()
        };

        let client_endpoint = transport::create_client_endpoint(bind_addr, client_tls)
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))?;

        let connection = client_endpoint
            .connect(addr, "continue-device")
            .map_err(|e| ContinueFfiError::InternalError(format!("Connect failed: {e}")))?
            .await
            .map_err(|e| ContinueFfiError::InternalError(format!("Handshake failed: {e}")))?;

        let mux = Arc::new(SessionMultiplexer::new(peer_fingerprint.clone(), connection));
        mux.spawn_keepalive_sender();

        let download_dir = std::env::temp_dir().join("continue_downloads");
        let _ = std::fs::create_dir_all(&download_dir);
        let handlers = sessions::SessionCapabilityHandlers::new(download_dir);
        sessions::spawn_capabilities_dispatcher(mux.clone(), handlers, 16);

        let mut lock = active_sessions.lock().unwrap();
        lock.insert(peer_fingerprint, mux);
        Ok(())
    })
}

pub fn is_peer_connected(peer_fingerprint: String) -> Result<bool, ContinueFfiError> {
    let lock = CORE.lock().unwrap();
    let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
    let sessions = state.active_sessions.lock().unwrap();
    Ok(sessions.contains_key(&peer_fingerprint))
}

pub fn disconnect(peer_fingerprint: String) -> Result<(), ContinueFfiError> {
    let (runtime, active_sessions, permission_store) = {
        let lock = CORE.lock().unwrap();
        let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
        (
            state.runtime.clone(),
            state.active_sessions.clone(),
            state.permission_store.clone(),
        )
    };

    permission_store.clear_allow_once_for_peer(&peer_fingerprint);

    let maybe_mux = {
        let mut lock = active_sessions.lock().unwrap();
        lock.remove(&peer_fingerprint)
    };

    if let Some(mux) = maybe_mux {
        runtime.block_on(async move {
            mux.disconnect(
                protocol::v1::DisconnectReason::DisconnectReasonNormal,
                "Disconnected by user".to_string(),
            )
            .await;
        });
    }

    Ok(())
}

pub fn send_file(peer_fingerprint: String, file_path: String) -> Result<u64, ContinueFfiError> {
    let (runtime, mux) = {
        let lock = CORE.lock().unwrap();
        let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
        let sessions = state.active_sessions.lock().unwrap();
        let m = sessions
            .get(&peer_fingerprint)
            .cloned()
            .ok_or_else(|| ContinueFfiError::InternalError("Peer not connected".to_string()))?;
        (state.runtime.clone(), m)
    };

    runtime.block_on(async move {
        let path = std::path::Path::new(&file_path);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let transfer_id = format!("tx-{now}");
        mux.send_file_to_peer(path, transfer_id)
            .await
            .map_err(|e| ContinueFfiError::InternalError(e.to_string()))
    })
}

pub fn send_clipboard_text(peer_fingerprint: String, text: String) -> Result<(), ContinueFfiError> {
    let (runtime, mux) = {
        let lock = CORE.lock().unwrap();
        let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
        let sessions = state.active_sessions.lock().unwrap();
        let m = sessions
            .get(&peer_fingerprint)
            .cloned()
            .ok_or_else(|| ContinueFfiError::InternalError("Peer not connected".to_string()))?;
        (state.runtime.clone(), m)
    };

    runtime.block_on(async move {
        let synchronizer = clipboard::ClipboardSynchronizer::new();
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
            &synchronizer,
            clipboard::ClipboardFormat::ClipboardFormatTextPlain,
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
    let (runtime, mux) = {
        let lock = CORE.lock().unwrap();
        let state = lock.as_ref().ok_or(ContinueFfiError::NotInitialized)?;
        let sessions = state.active_sessions.lock().unwrap();
        let m = sessions
            .get(&peer_fingerprint)
            .cloned()
            .ok_or_else(|| ContinueFfiError::InternalError("Peer not connected".to_string()))?;
        (state.runtime.clone(), m)
    };

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
