// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

use identity::{FileSecretStore, IdentitySigner};
use pairing::{DeviceKeys, InitiatorPairing, ReplayCache, TrustStore};
use permissions::PermissionStore;
use protocol::CapabilityId;
use sessions::{SessionRegistry, SessionState};
use transport::{DialConfig, TransportCertificate};

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
    trust_store: TrustStore,
    permission_store: Arc<PermissionStore>,
    replay_cache: Arc<ReplayCache>,
    allowed_spki_hashes: Arc<std::sync::RwLock<HashSet<[u8; 32]>>>,
    active_pairing: Arc<Mutex<Option<Arc<ActivePairingServer>>>>,
    sessions: SessionRegistry,
}

fn session_handlers(
    download_dir: PathBuf,
    permission_store: Arc<PermissionStore>,
    app_handle: &AppHandle,
) -> sessions::SessionCapabilityHandlers {
    let mut handlers = sessions::SessionCapabilityHandlers::new(download_dir)
        .with_permission_store(permission_store);

    let app_handle_files = app_handle.clone();
    handlers.on_file_received = Some(Arc::new(move |file| {
        let _ = app_handle_files.emit(
            "file-received",
            serde_json::json!({
                "fileName": file.file_name,
                "path": file.path.to_string_lossy(),
                "bytesReceived": file.bytes_received,
            }),
        );
    }));

    let app_handle_clips = app_handle.clone();
    handlers.on_clipboard_received = Some(Arc::new(move |update| {
        let text = String::from_utf8_lossy(&update.payload).to_string();
        let _ = app_handle_clips.emit(
            "clipboard-received",
            serde_json::json!({
                "format": format!("{:?}", update.format),
                "content": text,
            }),
        );
    }));

    handlers
}

fn session_state_listener(
    app_handle: AppHandle,
    trust_store: TrustStore,
) -> sessions::StateListener {
    Arc::new(move |peer, session_state| {
        let _ = app_handle.emit(
            "peer-state-changed",
            serde_json::json!({
                "fingerprint": peer,
                "state": session_state.to_string(),
            }),
        );
        if session_state == SessionState::Connected {
            let display_name = trust_store
                .get_peer(peer)
                .ok()
                .flatten()
                .map(|p| p.display_name)
                .unwrap_or_default();
            let _ = app_handle.emit(
                "peer-connected",
                serde_json::json!({
                    "fingerprint": peer,
                    "displayName": display_name,
                }),
            );
        }
    })
}

#[tauri::command]
fn get_device_identity(state: State<DesktopRuntimeState>) -> Result<DeviceIdentityDto, String> {
    let verifying_key = state
        .identity_signer
        .verifying_key()
        .map_err(|e| format!("Key derivation error: {e}"))?;
    let fingerprint = identity::Fingerprint::from_verifying_key(&verifying_key)
        .as_str()
        .to_string();
    let spki_hash = hex_encode(state.transport_cert.spki_hash);

    Ok(DeviceIdentityDto {
        device_name: state.device_name.clone(),
        fingerprint,
        spki_hash,
    })
}

#[tauri::command]
fn get_trusted_peers(state: State<DesktopRuntimeState>) -> Result<Vec<TrustedPeerDto>, String> {
    let peers = state
        .trust_store
        .list_peers()
        .map_err(|e| format!("Database error: {e}"))?;

    let dtos = peers
        .into_iter()
        .map(|p| {
            let is_connected = state.sessions.state(&p.fingerprint) == SessionState::Connected;
            let endpoint = state
                .trust_store
                .last_endpoint(&p.fingerprint)
                .ok()
                .flatten();
            TrustedPeerDto {
                fingerprint: p.fingerprint,
                display_name: p.display_name,
                paired_at: p.paired_at,
                is_connected,
                endpoint,
            }
        })
        .collect();

    Ok(dtos)
}

#[tauri::command]
async fn remove_trusted_peer(
    state: State<'_, DesktopRuntimeState>,
    fingerprint: String,
) -> Result<bool, String> {
    // Revoke trust before closing the session so the peer cannot reconnect in between.
    if let Ok(Some(peer)) = state.trust_store.get_peer(&fingerprint) {
        if let Ok(mut allowed) = state.allowed_spki_hashes.write() {
            allowed.remove(&peer.transport_spki_hash);
        }
    }

    let removed = state
        .trust_store
        .remove_peer(&fingerprint)
        .map_err(|e| format!("Database error: {e}"))?;
    state.sessions.remove(&fingerprint).await;
    Ok(removed)
}

/// Finds the address other devices on the LAN can reach us at. Connecting a UDP
/// socket only selects a route; no packet is sent, so this works offline as long
/// as the machine has a default route.
fn local_lan_ip() -> Result<std::net::IpAddr, String> {
    let socket =
        std::net::UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("Network unavailable: {e}"))?;
    socket
        .connect("192.0.2.1:9")
        .map_err(|e| format!("No local network route: {e}"))?;
    socket
        .local_addr()
        .map(|addr| addr.ip())
        .map_err(|e| format!("Network unavailable: {e}"))
}

#[tauri::command]
async fn start_pairing(
    app: AppHandle,
    state: State<'_, DesktopRuntimeState>,
) -> Result<String, String> {
    {
        let mut active = state.active_pairing.lock();
        if let Some(existing) = active.take() {
            existing.server_endpoint.close(0u32.into(), b"superseded");
        }
    }

    let recorded_spki: Arc<std::sync::Mutex<Option<[u8; 32]>>> =
        Arc::new(std::sync::Mutex::new(None));
    let server_tls = state
        .transport_cert
        .build_pairing_server_tls(recorded_spki.clone())
        .map_err(|e| format!("TLS configuration failed: {e}"))?;

    let lan_ip = local_lan_ip()?;
    let bind_addr = std::net::SocketAddr::from(([0, 0, 0, 0], 0));
    let server_endpoint = transport::create_server_endpoint(bind_addr, server_tls)
        .map_err(|e| format!("Failed to create server endpoint: {e}"))?;
    let listen_port = server_endpoint
        .local_addr()
        .map_err(|e| format!("Failed to read listening port: {e}"))?
        .port();
    let advertised_endpoint = std::net::SocketAddr::new(lan_ip, listen_port).to_string();

    let mut initiator = InitiatorPairing::new(
        state.identity_signer.clone(),
        state.transport_cert.clone(),
        state.trust_store.clone(),
        state.replay_cache.clone(),
    );

    let qr = initiator
        .generate_qr(advertised_endpoint)
        .map_err(|e| format!("Failed to generate pairing payload: {e}"))?;

    let pairing_server = Arc::new(ActivePairingServer {
        server_endpoint: server_endpoint.clone(),
    });
    *state.active_pairing.lock() = Some(pairing_server.clone());

    let endpoint_for_worker = server_endpoint.clone();
    let active_pairing = state.active_pairing.clone();
    let allowed_hashes = state.allowed_spki_hashes.clone();
    let trust_store = state.trust_store.clone();
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

        let result = initiator
            .complete_handshake(&mut send_stream, &mut recv_stream, recorded_hash)
            .await;
        if let Ok(peer) = &result {
            sessions::remember_peer_address(
                &trust_store,
                &peer.fingerprint,
                conn.remote_address().ip(),
            );
        }

        endpoint_for_worker.close(0u32.into(), b"pairing_finished");

        // A cancelled or superseded attempt must not report into the UI's current one.
        {
            let mut active = active_pairing.lock();
            if !active
                .as_ref()
                .is_some_and(|a| Arc::ptr_eq(a, &pairing_server))
            {
                return;
            }
            active.take();
        }

        let _ = match result {
            Ok(peer) => {
                if let Ok(mut allowed) = allowed_hashes.write() {
                    allowed.insert(peer.transport_spki_hash);
                }
                app.emit(
                    "pairing-completed",
                    TrustedPeerDto {
                        fingerprint: peer.fingerprint,
                        display_name: peer.display_name,
                        paired_at: peer.paired_at,
                        is_connected: false,
                        endpoint: None,
                    },
                )
            }
            Err(e) => app.emit("pairing-failed", format!("Pairing handshake failed: {e}")),
        };
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
    let qr =
        pairing::QrPayload::decode(&qr_payload).map_err(|e| format!("Invalid QR payload: {e}"))?;

    let addr: std::net::SocketAddr = qr
        .endpoint
        .parse()
        .map_err(|e| format!("Invalid endpoint address: {e}"))?;

    let connection = transport::connect_pinned(
        &state.transport_cert,
        qr.transport_spki_hash,
        addr,
        &DialConfig::default(),
    )
    .await
    .map_err(|e| format!("Connect failed: {e}"))?;

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

    if let Ok(mut allowed) = state.allowed_spki_hashes.write() {
        allowed.insert(trusted_peer.transport_spki_hash);
    }
    sessions::remember_peer_address(&state.trust_store, &trusted_peer.fingerprint, addr.ip());

    // The pairing connection is dropped here; the session is dialed at the saved address.
    Ok(TrustedPeerDto {
        fingerprint: trusted_peer.fingerprint,
        display_name: trusted_peer.display_name,
        paired_at: trusted_peer.paired_at,
        is_connected: false,
        endpoint: None,
    })
}

#[tauri::command]
fn get_permissions(
    state: State<DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<Vec<PeerPermissionDto>, String> {
    let store = &state.permission_store;
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
            state
                .permission_store
                .grant_allow_once(&peer_fingerprint, CapabilityId(capability_id));
            return Ok(());
        }
        _ => return Err(format!("Unsupported grant type: {grant}")),
    };

    state
        .permission_store
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
    let peer = state
        .trust_store
        .get_peer(&peer_fingerprint)
        .map_err(|e| format!("Database error: {e}"))?
        .ok_or_else(|| "Peer not found in trust store".to_string())?;

    let addr: std::net::SocketAddr = endpoint
        .parse()
        .map_err(|e| format!("Invalid endpoint address: {e}"))?;

    state
        .sessions
        .connect(&peer_fingerprint, peer.transport_spki_hash, addr)
        .await
        .map_err(|e| format!("Connect failed: {e}"))?;

    state
        .trust_store
        .set_last_endpoint(&peer_fingerprint, &addr.to_string())
        .map_err(|e| format!("Database error: {e}"))
}

#[tauri::command]
async fn disconnect_peer(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<(), String> {
    state.sessions.disconnect(&peer_fingerprint).await;
    Ok(())
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgressDto {
    pub bytes_sent: u64,
    pub total_bytes: u64,
}

/// Progress updates are capped so a fast LAN transfer doesn't flood the webview.
const PROGRESS_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

#[tauri::command]
async fn send_file_to_peer(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    file_path: String,
    on_progress: tauri::ipc::Channel<TransferProgressDto>,
) -> Result<u64, String> {
    let mux = state
        .sessions
        .get(&peer_fingerprint)
        .ok_or_else(|| "Peer is not connected".to_string())?;

    let path = std::path::PathBuf::from(file_path);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let transfer_id = format!("tx-{now}");

    let last_update = Mutex::new(std::time::Instant::now() - PROGRESS_INTERVAL);
    let report = |bytes_sent: u64, total_bytes: u64| {
        let mut last = last_update.lock();
        if bytes_sent < total_bytes && last.elapsed() < PROGRESS_INTERVAL {
            return;
        }
        *last = std::time::Instant::now();
        let _ = on_progress.send(TransferProgressDto {
            bytes_sent,
            total_bytes,
        });
    };

    mux.send_file_to_peer(&path, transfer_id, Some(report))
        .await
        .map_err(|e| format!("Failed to send file: {e}"))
}

#[tauri::command]
async fn send_clipboard_text(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    text: String,
) -> Result<(), String> {
    let mux = state
        .sessions
        .get(&peer_fingerprint)
        .ok_or_else(|| "Peer is not connected".to_string())?;

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
    let mux = state
        .sessions
        .get(&peer_fingerprint)
        .ok_or_else(|| "Peer is not connected".to_string())?;

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

fn initialize_desktop_runtime(
    app_handle: &AppHandle,
    db_path: &Path,
    secrets_dir: &Path,
    download_dir: PathBuf,
) -> Result<DesktopRuntimeState, Box<dyn std::error::Error>> {
    let trust_store = TrustStore::open(db_path)?;
    let permission_store = Arc::new(PermissionStore::open(db_path)?);

    let DeviceKeys {
        identity_signer,
        transport_cert,
    } = DeviceKeys::load_or_create(&FileSecretStore::new(secrets_dir)?)?;

    let peers = trust_store.list_peers()?;
    let initial_hashes: HashSet<[u8; 32]> = peers.iter().map(|p| p.transport_spki_hash).collect();
    let allowed_spki_hashes = Arc::new(std::sync::RwLock::new(initial_hashes));

    let local_fingerprint =
        identity::Fingerprint::from_verifying_key(&identity_signer.verifying_key()?).to_string();
    let sessions = SessionRegistry::new(
        local_fingerprint,
        transport_cert.clone(),
        session_handlers(download_dir, permission_store.clone(), app_handle),
        sessions::RegistryConfig::default(),
        Some(session_state_listener(
            app_handle.clone(),
            trust_store.clone(),
        )),
    );

    let device_name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "Desktop PC".to_string());

    Ok(DesktopRuntimeState {
        device_name,
        identity_signer,
        transport_cert,
        trust_store,
        permission_store,
        replay_cache: Arc::new(ReplayCache::new()),
        allowed_spki_hashes,
        active_pairing: Arc::new(Mutex::new(None)),
        sessions,
    })
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let _ = std::fs::create_dir_all(&app_data);

            let db_path = app_data.join("continue_desktop.db");
            let secrets_dir = app_data.join("secrets");
            let _ = std::fs::create_dir_all(&secrets_dir);

            let download_dir = app
                .path()
                .download_dir()
                .unwrap_or_else(|_| app_data.join("downloads"));
            let _ = std::fs::create_dir_all(&download_dir);

            let runtime_state =
                initialize_desktop_runtime(app.handle(), &db_path, &secrets_dir, download_dir)
                    .expect("Failed to initialize Continue desktop runtime engine");

            // quinn needs a running async runtime to open the endpoint, which `setup` doesn't have.
            let cert = runtime_state.transport_cert.clone();
            let trusted_keys = runtime_state.allowed_spki_hashes.clone();
            let sessions = runtime_state.sessions.clone();
            let trust_store = runtime_state.trust_store.clone();
            tauri::async_runtime::spawn(async move {
                let listener = sessions::listen_for_peers(&cert, trusted_keys)
                    .and_then(|endpoint| Ok((endpoint.local_addr()?.port(), endpoint)));
                let (port, endpoint) = match listener {
                    Ok(listener) => listener,
                    Err(error) => {
                        tracing::error!("Could not listen for paired devices: {error}");
                        return;
                    }
                };
                tracing::info!("Listening for paired devices on port {port}");
                tauri::async_runtime::spawn(discovery::advertise(port, 1));
                tauri::async_runtime::spawn(sessions::connect_paired_peers(
                    sessions.clone(),
                    trust_store.clone(),
                ));
                sessions::accept_peers(endpoint, sessions, trust_store).await;
            });

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
