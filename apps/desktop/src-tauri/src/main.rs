// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use identity::IdentitySigner;
use pairing::{InitiatorPairing, ReplayCache, TrustStore};
use permissions::PermissionStore;
use protocol::CapabilityId;
use sessions::SessionMultiplexer;
use transport::TransportCertificate;

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentityDto {
    pub device_name: String,
    pub fingerprint: String,
    pub spki_hash: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TrustedPeerDto {
    pub fingerprint: String,
    pub display_name: String,
    pub paired_at: u64,
    pub is_connected: bool,
    pub endpoint: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PeerPermissionDto {
    pub capability_id: u32,
    pub capability_name: String,
    pub grant: String,
}

struct ActivePairingServer {
    server_endpoint: quinn::Endpoint,
}

pub struct DesktopRuntimeState {
    device_name: String,
    identity_signer: Arc<dyn IdentitySigner>,
    transport_cert: Arc<TransportCertificate>,
    trust_store: Arc<Mutex<TrustStore>>,
    permission_store: Arc<Mutex<PermissionStore>>,
    replay_cache: Arc<ReplayCache>,
    active_pairing: Arc<Mutex<Option<ActivePairingServer>>>,
    active_sessions: Arc<Mutex<HashMap<String, Arc<SessionMultiplexer>>>>,
}

#[tauri::command]
fn get_device_identity(state: State<DesktopRuntimeState>) -> Result<DeviceIdentityDto, String> {
    let verifying_key = state
        .identity_signer
        .verifying_key()
        .map_err(|e| format!("Key derivation error: {e}"))?;
    let fingerprint = identity::Fingerprint::from_verifying_key(&verifying_key).as_str().to_string();
    let spki_hash = hex_encode(state.transport_cert.spki_hash);

    Ok(DeviceIdentityDto {
        device_name: state.device_name.clone(),
        fingerprint,
        spki_hash,
    })
}

#[tauri::command]
fn get_trusted_peers(state: State<DesktopRuntimeState>) -> Result<Vec<TrustedPeerDto>, String> {
    let peers = {
        let store = state.trust_store.lock();
        store.list_peers().map_err(|e| format!("Database error: {e}"))?
    };

    let sessions = state.active_sessions.lock();
    let dtos = peers
        .into_iter()
        .map(|p| {
            let is_connected = sessions.contains_key(&p.fingerprint);
            TrustedPeerDto {
                fingerprint: p.fingerprint,
                display_name: p.display_name,
                paired_at: p.paired_at,
                is_connected,
                endpoint: None,
            }
        })
        .collect();

    Ok(dtos)
}

#[tauri::command]
fn remove_trusted_peer(state: State<DesktopRuntimeState>, fingerprint: String) -> Result<bool, String> {
    {
        let mut sessions = state.active_sessions.lock();
        if let Some(session) = sessions.remove(&fingerprint) {
            session.connection().close(0u32.into(), b"peer_removed");
        }
    }

    let mut store = state.trust_store.lock();
    store.remove_peer(&fingerprint).map_err(|e| format!("Database error: {e}"))
}

#[tauri::command]
async fn start_pairing(
    state: State<'_, DesktopRuntimeState>,
    listen_port: u16,
    advertised_endpoint: String,
) -> Result<String, String> {
    {
        let mut active = state.active_pairing.lock();
        if let Some(existing) = active.take() {
            existing.server_endpoint.close(0u32.into(), b"superseded");
        }
    }

    let recorded_spki: Arc<std::sync::Mutex<Option<[u8; 32]>>> = Arc::new(std::sync::Mutex::new(None));
    let server_tls = state
        .transport_cert
        .build_pairing_server_tls(recorded_spki.clone())
        .map_err(|e| format!("TLS configuration failed: {e}"))?;

    let bind_addr = std::net::SocketAddr::from(([0, 0, 0, 0], listen_port));
    let server_endpoint = transport::create_server_endpoint(bind_addr, server_tls)
        .map_err(|e| format!("Failed to create server endpoint: {e}"))?;

    let mut initiator = InitiatorPairing::new(
        state.identity_signer.clone(),
        state.transport_cert.clone(),
        state.trust_store.clone(),
        state.replay_cache.clone(),
    );

    let qr = initiator
        .generate_qr(advertised_endpoint)
        .map_err(|e| format!("Failed to generate pairing payload: {e}"))?;

    let pairing_server = ActivePairingServer {
        server_endpoint: server_endpoint.clone(),
    };
    *state.active_pairing.lock() = Some(pairing_server);

    let endpoint_for_worker = server_endpoint.clone();
    tokio::spawn(async move {
        let incoming = match endpoint_for_worker.accept().await {
            Some(inc) => inc,
            None => return,
        };

        let conn = match incoming.await {
            Ok(c) => c,
            Err(_) => return,
        };

        let (mut send_stream, mut recv_stream) = match conn.accept_bi().await {
            Ok(s) => s,
            Err(_) => return,
        };

        let recorded_hash = match *recorded_spki.lock().unwrap() {
            Some(h) => h,
            None => return,
        };

        let _ = initiator
            .complete_handshake(&mut send_stream, &mut recv_stream, recorded_hash)
            .await;
    });

    Ok(qr.encode())
}

#[tauri::command]
fn cancel_pairing(state: State<DesktopRuntimeState>) -> Result<(), String> {
    let mut active = state.active_pairing.lock();
    if let Some(existing) = active.take() {
        existing.server_endpoint.close(0u32.into(), b"cancelled");
    }
    Ok(())
}

#[tauri::command]
async fn pair_from_qr(
    state: State<'_, DesktopRuntimeState>,
    qr_payload: String,
) -> Result<TrustedPeerDto, String> {
    let qr = pairing::QrPayload::decode(&qr_payload)
        .map_err(|e| format!("Invalid QR payload: {e}"))?;

    let addr: std::net::SocketAddr = qr
        .endpoint
        .parse()
        .map_err(|e| format!("Invalid endpoint address: {e}"))?;

    let client_tls = state
        .transport_cert
        .build_pinned_client_tls(qr.transport_spki_hash)
        .map_err(|e| format!("TLS configuration failed: {e}"))?;

    let bind_addr: std::net::SocketAddr = if addr.is_ipv6() {
        "[::]:0".parse().unwrap()
    } else {
        "0.0.0.0:0".parse().unwrap()
    };

    let client_endpoint = transport::create_client_endpoint(bind_addr, client_tls)
        .map_err(|e| format!("Failed to create client endpoint: {e}"))?;

    let connection = client_endpoint
        .connect(addr, "continue-device")
        .map_err(|e| format!("Connect failed: {e}"))?
        .await
        .map_err(|e| format!("TLS handshake failed: {e}"))?;

    let (mut send_stream, mut recv_stream) = connection
        .open_bi()
        .await
        .map_err(|e| format!("Stream open failed: {e}"))?;

    let responder = pairing::ResponderPairing::new(
        state.identity_signer.clone(),
        state.transport_cert.clone(),
        state.trust_store.clone(),
    );

    let trusted_peer = responder
        .complete_handshake(&qr, &mut send_stream, &mut recv_stream)
        .await
        .map_err(|e| format!("Pairing handshake failed: {e}"))?;

    Ok(TrustedPeerDto {
        fingerprint: trusted_peer.fingerprint,
        display_name: trusted_peer.display_name,
        paired_at: trusted_peer.paired_at,
        is_connected: true,
        endpoint: Some(qr.endpoint),
    })
}

#[tauri::command]
fn get_permissions(
    state: State<DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<Vec<PeerPermissionDto>, String> {
    let store = state.permission_store.lock();
    let capabilities = [
        (CapabilityId::FILE_TRANSFER, "File Transfer"),
        (CapabilityId::CLIPBOARD, "Clipboard Sync"),
        (CapabilityId::NOTIFICATIONS, "Notification Relay"),
    ];

    let mut list = Vec::with_capacity(capabilities.len());
    for (cap_id, name) in capabilities {
        let perm_state = store
            .query_state(&peer_fingerprint, cap_id)
            .map_err(|e| format!("Database error: {e}"))?;

        let grant_str = match perm_state {
            permissions::PermissionState::Allow => "Allow",
            permissions::PermissionState::Deny => "Deny",
            permissions::PermissionState::Ask => "Ask",
            permissions::PermissionState::AllowOnce => "AllowOnce",
        };

        list.push(PeerPermissionDto {
            capability_id: cap_id.raw(),
            capability_name: name.to_string(),
            grant: grant_str.to_string(),
        });
    }

    Ok(list)
}

#[tauri::command]
fn set_permission(
    state: State<DesktopRuntimeState>,
    peer_fingerprint: String,
    capability_id: u32,
    grant: String,
) -> Result<(), String> {
    let parsed_grant = match grant.as_str() {
        "Allow" => permissions::PersistedGrant::Allow,
        "Deny" => permissions::PersistedGrant::Deny,
        "Ask" => permissions::PersistedGrant::Ask,
        "AllowOnce" => {
            let mut store = state.permission_store.lock();
            store.grant_allow_once(&peer_fingerprint, CapabilityId(capability_id));
            return Ok(());
        }
        _ => return Err(format!("Unsupported grant type: {grant}")),
    };

    let mut store = state.permission_store.lock();
    store
        .set_persisted_grant(
            &peer_fingerprint,
            CapabilityId(capability_id),
            1,
            parsed_grant,
        )
        .map_err(|e| format!("Database error: {e}"))?;

    Ok(())
}

#[tauri::command]
async fn connect_to_peer(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    endpoint: String,
) -> Result<(), String> {
    let peer = {
        let store = state.trust_store.lock();
        store
            .get_peer(&peer_fingerprint)
            .map_err(|e| format!("Database error: {e}"))?
            .ok_or_else(|| "Peer not found in trust store".to_string())?
    };

    let addr: std::net::SocketAddr = endpoint
        .parse()
        .map_err(|e| format!("Invalid endpoint address: {e}"))?;

    let client_tls = state
        .transport_cert
        .build_pinned_client_tls(peer.transport_spki_hash)
        .map_err(|e| format!("TLS configuration failed: {e}"))?;

    let bind_addr: std::net::SocketAddr = if addr.is_ipv6() {
        "[::]:0".parse().unwrap()
    } else {
        "0.0.0.0:0".parse().unwrap()
    };

    let client_endpoint = transport::create_client_endpoint(bind_addr, client_tls)
        .map_err(|e| format!("Failed to create client endpoint: {e}"))?;

    let connection = client_endpoint
        .connect(addr, "continue-device")
        .map_err(|e| format!("Connect failed: {e}"))?
        .await
        .map_err(|e| format!("TLS handshake failed: {e}"))?;

    let mux = Arc::new(SessionMultiplexer::new(peer_fingerprint.clone(), connection));
    mux.spawn_keepalive_sender();

    let download_dir = std::env::temp_dir().join("continue_desktop_downloads");
    let _ = std::fs::create_dir_all(&download_dir);
    let handlers = sessions::SessionCapabilityHandlers::new(download_dir);
    sessions::spawn_capabilities_dispatcher(mux.clone(), handlers, 16);

    let mut sessions = state.active_sessions.lock();
    sessions.insert(peer_fingerprint, mux);

    Ok(())
}

#[tauri::command]
fn disconnect_peer(state: State<DesktopRuntimeState>, peer_fingerprint: String) -> Result<(), String> {
    let mut sessions = state.active_sessions.lock();
    if let Some(session) = sessions.remove(&peer_fingerprint) {
        session.connection().close(0u32.into(), b"user_disconnect");
    }
    Ok(())
}

#[tauri::command]
async fn send_file_to_peer(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    file_path: String,
) -> Result<u64, String> {
    let mux = {
        let sessions = state.active_sessions.lock();
        sessions
            .get(&peer_fingerprint)
            .cloned()
            .ok_or_else(|| "Peer is not connected".to_string())?
    };

    let path = std::path::PathBuf::from(file_path);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let transfer_id = format!("tx-{now}");

    mux.send_file_to_peer(&path, transfer_id)
        .await
        .map_err(|e| format!("Failed to send file: {e}"))
}

#[tauri::command]
async fn send_clipboard_text(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    text: String,
) -> Result<(), String> {
    let mux = {
        let sessions = state.active_sessions.lock();
        sessions
            .get(&peer_fingerprint)
            .cloned()
            .ok_or_else(|| "Peer is not connected".to_string())?
    };

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
    .map_err(|e| format!("Clipboard sync error: {e}"))?;

    Ok(())
}

#[tauri::command]
async fn send_notification(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    title: String,
    body: String,
    app_name: String,
) -> Result<(), String> {
    let mux = {
        let sessions = state.active_sessions.lock();
        sessions
            .get(&peer_fingerprint)
            .cloned()
            .ok_or_else(|| "Peer is not connected".to_string())?
    };

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
        package_name: "continue.desktop".to_string(),
        app_name,
        title,
        body,
        timestamp: now,
        actions: vec![],
    };

    mux.send_notification_to_peer(&dispatcher, post, &query)
        .await
        .map_err(|e| format!("Notification error: {e}"))?;

    Ok(())
}

fn initialize_desktop_runtime(db_path: &str) -> Result<DesktopRuntimeState, Box<dyn std::error::Error>> {
    let trust_store = Arc::new(Mutex::new(TrustStore::open(db_path)?));
    let permission_store = Arc::new(Mutex::new(PermissionStore::open(db_path)?));
    let transport_cert = Arc::new(TransportCertificate::generate()?);

    let seed = crypto::keys::generate_ed25519_seed();
    let signing_key = crypto::keys::signing_key_from_seed(&seed.0);
    let identity_signer: Arc<dyn IdentitySigner> = Arc::new(identity::InMemorySigner::new(signing_key));

    Ok(DesktopRuntimeState {
        device_name: "Desktop PC".to_string(),
        identity_signer,
        transport_cert,
        trust_store,
        permission_store,
        replay_cache: Arc::new(ReplayCache::new()),
        active_pairing: Arc::new(Mutex::new(None)),
        active_sessions: Arc::new(Mutex::new(HashMap::new())),
    })
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let db_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
            let _ = std::fs::create_dir_all(&db_dir);
            let db_path = db_dir.join("continue_desktop.db");
            let db_path_str = db_path.to_string_lossy().to_string();

            let runtime_state = initialize_desktop_runtime(&db_path_str)
                .expect("Failed to initialize Continue desktop runtime engine");

            app.manage(runtime_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_device_identity,
            get_trusted_peers,
            remove_trusted_peer,
            start_pairing,
            cancel_pairing,
            pair_from_qr,
            get_permissions,
            set_permission,
            connect_to_peer,
            disconnect_peer,
            send_file_to_peer,
            send_clipboard_text,
            send_notification
        ])
        .run(tauri::generate_context!())
        .expect("error while running Continue desktop application");
}
