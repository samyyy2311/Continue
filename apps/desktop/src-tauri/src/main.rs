// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard_sync;
mod receiving;
mod secrets;
mod tray;

use clipboard_sync::ClipboardSync;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use history::{Direction, HistoryStore, Kind};
use identity::IdentitySigner;
use pairing::{DeviceKeys, InitiatorPairing, ReplayCache, TrustStore};
use permissions::PermissionStore;
use protocol::CapabilityId;
use sessions::{PermissionDecision, SessionRegistry, SessionState};
use transport::{DialConfig, TransportCertificate};

/// Logs what actually went wrong and gives the UI a sentence a person can act on.
fn user_error<E: std::fmt::Display>(message: &'static str) -> impl FnOnce(E) -> String {
    move |error| {
        tracing::warn!("{message} ({error})");
        message.to_string()
    }
}

const NOT_CONNECTED: &str = "That device isn't connected.";
const PAIRING_SETUP_FAILED: &str = "Couldn't get a pairing code ready. Try again.";
const PAIRING_FAILED: &str = "Pairing didn't finish. Try again.";
const BAD_CODE: &str = "That code doesn't look right. Copy the whole code and try again.";
const UNREACHABLE: &str = "Couldn't reach the other device. Check that both are on the same Wi-Fi.";
const NO_NETWORK: &str = "This computer isn't on a network. Connect to Wi-Fi and try again.";
const SAVE_FAILED: &str = "Couldn't save that change. Try again.";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentityDto {
    pub device_name: String,
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

/// A question for the window, as it shows it.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PermissionQuestionDto {
    pub id: u64,
    pub peer_name: String,
    pub kind: &'static str,
    pub detail: Option<String>,
}

struct PendingQuestion {
    question: PermissionQuestionDto,
    answer: tokio::sync::oneshot::Sender<PermissionDecision>,
}

/// Questions waiting for the user, by the id the window answers with.
type PendingAnswers = Arc<Mutex<HashMap<u64, PendingQuestion>>>;

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
    pending_answers: PendingAnswers,
    history: HistoryStore,
    save_folder: sessions::SaveFolder,
    incoming: sessions::IncomingFiles,
    clipboard: ClipboardSync,
}

/// Saves to history; a failure there shouldn't fail what was actually done.
fn remember(history: &HistoryStore, item: history::Item) {
    if let Err(error) = history.record(&item) {
        tracing::warn!("Couldn't save to history: {error}");
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntryDto {
    pub id: i64,
    pub at: u64,
    pub received: bool,
    pub kind: &'static str,
    pub label: String,
    pub peer_id: String,
    pub peer_name: String,
    pub size: u64,
    pub failed: bool,
    pub location: Option<String>,
}

#[tauri::command]
fn get_history(state: State<DesktopRuntimeState>) -> Result<Vec<HistoryEntryDto>, String> {
    let entries = state
        .history
        .list(history::HISTORY_LIMIT)
        .map_err(user_error("Couldn't load your history."))?;
    Ok(entries
        .into_iter()
        .map(|entry| HistoryEntryDto {
            id: entry.id,
            at: entry.at,
            received: entry.item.direction == Direction::Received,
            kind: match entry.item.kind {
                Kind::File => "file",
                Kind::Text => "text",
            },
            label: entry.item.label,
            peer_id: entry.item.peer_fingerprint,
            peer_name: entry.item.peer_name,
            size: entry.item.size,
            failed: entry.item.failed,
            location: entry.item.location,
        })
        .collect())
}

#[tauri::command]
fn clear_history(state: State<DesktopRuntimeState>) -> Result<(), String> {
    state
        .history
        .clear()
        .map_err(user_error("Couldn't clear your history."))
}

/// Opens a received file, or shows it in its folder. Only files in the
/// received-files folder can be opened this way.
#[tauri::command]
fn open_received(
    app: AppHandle,
    state: State<DesktopRuntimeState>,
    path: String,
    reveal: bool,
) -> Result<(), String> {
    const MISSING: &str = "Couldn't find that file. It may have been moved or deleted.";
    let file = std::fs::canonicalize(&path).map_err(user_error(MISSING))?;
    let folder = std::fs::canonicalize(state.save_folder.get()).map_err(user_error(MISSING))?;
    // Files from before the folder was changed are still in history, so they can open too.
    if !file.starts_with(&folder) && !was_received(&state.history, &file) {
        return Err(MISSING.to_string());
    }
    let opener = app.opener();
    let result = if reveal {
        opener.reveal_item_in_dir(&file)
    } else {
        opener.open_path(file.to_string_lossy(), None::<&str>)
    };
    result.map_err(user_error("Couldn't open that file."))
}

fn was_received(history: &HistoryStore, file: &Path) -> bool {
    let entries = history.list(history::HISTORY_LIMIT).unwrap_or_default();
    entries.iter().any(|entry| {
        entry.item.direction == Direction::Received
            && entry
                .item
                .location
                .as_deref()
                .and_then(|location| std::fs::canonicalize(location).ok())
                .is_some_and(|location| location == file)
    })
}

/// Opens a web link that was sent or received. Anything but http and https is refused.
#[tauri::command]
fn open_link(app: AppHandle, url: String) -> Result<(), String> {
    const FAILED: &str = "Couldn't open that link.";
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(FAILED.to_string());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(user_error(FAILED))
}

/// Started at login with this, Continue stays in the tray instead of opening its window.
const BACKGROUND_ARG: &str = "--background";

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(user_error("Couldn't change that setting."))
}

/// Starts Continue at login the first time it runs. After that it's the user's choice.
fn start_at_login_by_default(app: &AppHandle, app_data: &Path) {
    let chosen = app_data.join("start-at-login-set");
    if chosen.exists() {
        return;
    }
    match app.autolaunch().enable() {
        Ok(()) => {
            let _ = std::fs::write(chosen, b"");
        }
        Err(error) => tracing::warn!("Couldn't start Continue at login: {error}"),
    }
}

/// Where pasted files wait to be sent. Emptied on each start.
fn pasted_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_cache_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("continue"))
        .join("pasted")
}

/// Saves a pasted file so it can be sent like one picked from disk. The
/// bytes are the request body and the name is in the `x-file-name` header.
#[tauri::command]
fn save_pasted_file(app: AppHandle, request: tauri::ipc::Request) -> Result<String, String> {
    const FAILED: &str = "Couldn't send what you pasted.";
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(FAILED.to_string());
    };
    let name = request
        .headers()
        .get("x-file-name")
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            percent_encoding::percent_decode_str(value)
                .decode_utf8_lossy()
                .into_owned()
        })
        .unwrap_or_default();
    let name = Path::new(&name).file_name().map_or_else(
        || "Pasted file".into(),
        |n| n.to_string_lossy().into_owned(),
    );
    // A folder per paste keeps the original name without clashing.
    let folder = pasted_dir(&app).join(NEXT.fetch_add(1, Ordering::Relaxed).to_string());
    let path = folder.join(name);
    std::fs::create_dir_all(&folder)
        .and_then(|()| std::fs::write(&path, bytes))
        .map_err(user_error(FAILED))?;
    Ok(path.to_string_lossy().into_owned())
}

/// The name people gave this computer: the "pretty" name where the OS has one (macOS's
/// Computer Name, Linux's pretty hostname), otherwise the hostname.
fn computer_name() -> String {
    [whoami::devicename(), whoami::hostname()]
        .into_iter()
        .filter_map(Result::ok)
        .map(|name| sessions::clean_name(&name))
        .find(|name| !name.is_empty())
        .unwrap_or_else(|| "Computer".to_string())
}

fn peer_name(trust_store: &TrustStore, fingerprint: &str) -> String {
    trust_store
        .get_peer(fingerprint)
        .ok()
        .flatten()
        .map(|p| p.display_name)
        .unwrap_or_default()
}

/// Shows a system notification, but only while the window isn't in front, where the same
/// news already shows.
fn notify_if_away(app: &AppHandle, title: &str, body: &str) {
    let in_front = app
        .get_webview_window("main")
        .and_then(|window| window.is_focused().ok())
        .unwrap_or(false);
    if in_front {
        return;
    }
    if let Err(error) = app.notification().builder().title(title).body(body).show() {
        tracing::warn!("Couldn't show a notification: {error}");
    }
}

/// The name to show for a paired device, even one saved without a name.
fn shown_name(name: String) -> String {
    if name.is_empty() {
        "Your phone".to_string()
    } else {
        name
    }
}

fn session_handlers(
    download_dir: PathBuf,
    permission_store: Arc<PermissionStore>,
    trust_store: TrustStore,
    pending_answers: PendingAnswers,
    history: HistoryStore,
    clipboard: ClipboardSync,
    app_handle: &AppHandle,
) -> sessions::SessionCapabilityHandlers {
    let mut handlers = sessions::SessionCapabilityHandlers::new(download_dir)
        .with_this_device(sessions::ThisDevice::new(
            computer_name(),
            sessions::this_platform(),
        ))
        .with_permission_store(permission_store)
        .with_permission_prompt(permission_prompt(
            app_handle.clone(),
            trust_store.clone(),
            pending_answers,
        ))
        .with_incoming(sessions::IncomingFiles::with_listener(receiving::listener(
            app_handle.clone(),
            trust_store.clone(),
        )));

    let (app, peers, saved) = (app_handle.clone(), trust_store.clone(), history.clone());
    handlers.on_file_received = Some(Arc::new(move |peer, file| {
        let name = peer_name(&peers, peer);
        notify_if_away(
            &app,
            &format!("{} sent a file", shown_name(name.clone())),
            &file.file_name,
        );
        remember(
            &saved,
            history::Item {
                direction: Direction::Received,
                kind: Kind::File,
                label: file.file_name.clone(),
                peer_fingerprint: peer.to_string(),
                peer_name: name.clone(),
                size: file.bytes_received,
                failed: false,
                location: Some(file.path.to_string_lossy().into_owned()),
            },
        );
        let _ = app.emit(
            "file-received",
            serde_json::json!({
                "peerId": peer,
                "peerName": name,
                "fileName": file.file_name,
                "path": file.path.to_string_lossy(),
                "bytesReceived": file.bytes_received,
            }),
        );
    }));

    let app = app_handle.clone();
    handlers.on_clipboard_received = Some(Arc::new(move |peer, update| {
        let text = String::from_utf8_lossy(&update.payload);
        clipboard.write(text.to_string());
        let name = peer_name(&trust_store, peer);
        notify_if_away(
            &app,
            &format!("Copied text from {}", shown_name(name.clone())),
            text.lines().next().unwrap_or_default(),
        );
        remember(
            &history,
            history::Item {
                direction: Direction::Received,
                kind: Kind::Text,
                label: text.to_string(),
                peer_fingerprint: peer.to_string(),
                peer_name: name.clone(),
                size: update.payload.len() as u64,
                failed: false,
                location: None,
            },
        );
        let _ = app.emit(
            "clipboard-received",
            serde_json::json!({
                "peerId": peer,
                "peerName": name,
                "content": text,
            }),
        );
    }));

    handlers
}

/// Saves the name a device sends when it connects, and shows it in the window and tray.
fn device_info_listener(
    app: AppHandle,
    trust_store: TrustStore,
    connected: Arc<tray::Connected>,
) -> sessions::OnReceived<sessions::PeerDevice> {
    Arc::new(
        move |peer, device| match trust_store.set_display_name(peer, &device.name) {
            Ok(true) => {
                connected.rename(&app, peer, device.name);
                let _ = app.emit("peer-renamed", peer);
            }
            Ok(false) => {}
            Err(error) => tracing::warn!("Couldn't save the name of {peer}: {error}"),
        },
    )
}

/// Asks in the window, which answers through `answer_permission`. The core stops waiting
/// after `PROMPT_TIMEOUT`, and so does the window.
fn permission_prompt(
    app_handle: AppHandle,
    trust_store: TrustStore,
    pending_answers: PendingAnswers,
) -> sessions::PermissionPrompt {
    let next_id = AtomicU64::new(0);
    Arc::new(move |request| {
        let (answer, decision) = tokio::sync::oneshot::channel();
        let question = PermissionQuestionDto {
            id: next_id.fetch_add(1, Ordering::Relaxed),
            peer_name: peer_name(&trust_store, &request.peer),
            kind: match request.capability {
                CapabilityId::FILE_TRANSFER => "file",
                CapabilityId::CLIPBOARD => "text",
                _ => "notification",
            },
            detail: request.detail,
        };
        let id = question.id;
        pending_answers.lock().insert(
            id,
            PendingQuestion {
                question: question.clone(),
                answer,
            },
        );
        let what = match (question.detail.as_deref(), question.kind) {
            (Some(file), _) => file,
            (None, "text") => "some text",
            (None, _) => "notifications",
        };
        notify_if_away(
            &app_handle,
            &format!(
                "{} wants to send {what}",
                shown_name(question.peer_name.clone())
            ),
            "Open Continue to allow or decline.",
        );
        let _ = app_handle.emit("permission-request", question);

        let (pending, app) = (pending_answers.clone(), app_handle.clone());
        tokio::spawn(async move {
            tokio::time::sleep_until(request.deadline).await;
            if pending.lock().remove(&id).is_some() {
                let _ = app.emit("permission-request-closed", id);
            }
        });
        decision
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum Answer {
    Allow,
    AlwaysAllow,
    Decline,
}

#[tauri::command]
fn answer_permission(state: State<DesktopRuntimeState>, id: u64, answer: Answer) {
    let decision = match answer {
        Answer::Allow => PermissionDecision::Allow,
        Answer::AlwaysAllow => PermissionDecision::AlwaysAllow,
        Answer::Decline => PermissionDecision::Decline,
    };
    if let Some(waiting) = state.pending_answers.lock().remove(&id) {
        let _ = waiting.answer.send(decision);
    }
}

/// Questions asked before the window was listening, so none go unseen.
#[tauri::command]
fn pending_permission_questions(state: State<DesktopRuntimeState>) -> Vec<PermissionQuestionDto> {
    let mut questions: Vec<_> = state
        .pending_answers
        .lock()
        .values()
        .map(|pending| pending.question.clone())
        .collect();
    questions.sort_by_key(|q| q.id);
    questions
}

fn session_state_listener(
    app_handle: AppHandle,
    trust_store: TrustStore,
    connected: Arc<tray::Connected>,
) -> sessions::StateListener {
    Arc::new(move |peer, session_state| {
        let is_connected = session_state == SessionState::Connected;
        connected.update(
            &app_handle,
            peer,
            shown_name(peer_name(&trust_store, peer)),
            is_connected,
        );
        let _ = app_handle.emit(
            "peer-state-changed",
            serde_json::json!({
                "fingerprint": peer,
                "state": session_state.to_string(),
            }),
        );
        if is_connected {
            let _ = app_handle.emit(
                "peer-connected",
                serde_json::json!({
                    "fingerprint": peer,
                    "displayName": shown_name(peer_name(&trust_store, peer)),
                }),
            );
        }
    })
}

#[tauri::command]
fn get_device_identity(state: State<DesktopRuntimeState>) -> DeviceIdentityDto {
    DeviceIdentityDto {
        device_name: state.device_name.clone(),
    }
}

#[tauri::command]
fn get_trusted_peers(state: State<DesktopRuntimeState>) -> Result<Vec<TrustedPeerDto>, String> {
    let peers = state
        .trust_store
        .list_peers()
        .map_err(user_error("Couldn't load your devices."))?;

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
                display_name: shown_name(p.display_name),
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
        .map_err(user_error("Couldn't forget this device. Try again."))?;
    state.sessions.remove(&fingerprint).await;
    Ok(removed)
}

/// Finds the address other devices on the LAN can reach us at. Connecting a UDP
/// socket only selects a route; no packet is sent, so this works offline as long
/// as the machine has a default route.
fn local_lan_ip() -> Result<std::net::IpAddr, String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").map_err(user_error(NO_NETWORK))?;
    socket
        .connect("192.0.2.1:9")
        .map_err(user_error(NO_NETWORK))?;
    socket
        .local_addr()
        .map(|addr| addr.ip())
        .map_err(user_error(NO_NETWORK))
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
        .map_err(user_error(PAIRING_SETUP_FAILED))?;

    let lan_ip = local_lan_ip()?;
    let bind_addr = std::net::SocketAddr::from(([0, 0, 0, 0], 0));
    let server_endpoint = transport::create_server_endpoint(bind_addr, server_tls)
        .map_err(user_error(PAIRING_SETUP_FAILED))?;
    let listen_port = server_endpoint
        .local_addr()
        .map_err(user_error(PAIRING_SETUP_FAILED))?
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
        .map_err(user_error(PAIRING_SETUP_FAILED))?;

    let pairing_server = Arc::new(ActivePairingServer {
        server_endpoint: server_endpoint.clone(),
    });
    *state.active_pairing.lock() = Some(pairing_server.clone());

    let endpoint_for_worker = server_endpoint.clone();
    let active_pairing = state.active_pairing.clone();
    let allowed_hashes = state.allowed_spki_hashes.clone();
    let trust_store = state.trust_store.clone();
    let registry = state.sessions.clone();
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
            // Trust the new key before redialing, and even if a newer attempt replaced this one,
            // or the listener turns the phone away.
            if let Ok(mut allowed) = allowed_hashes.write() {
                allowed.insert(peer.transport_spki_hash);
            }
            sessions::remember_peer_address(
                &trust_store,
                &peer.fingerprint,
                conn.remote_address().ip(),
            );
            registry.redial_now();
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
            Ok(peer) => app.emit(
                "pairing-completed",
                TrustedPeerDto {
                    fingerprint: peer.fingerprint,
                    display_name: shown_name(peer.display_name),
                    paired_at: peer.paired_at,
                    is_connected: false,
                    endpoint: None,
                },
            ),
            Err(e) => app.emit("pairing-failed", user_error(PAIRING_FAILED)(e)),
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
    let qr = pairing::QrPayload::decode(&qr_payload).map_err(user_error(BAD_CODE))?;

    let addr: std::net::SocketAddr = qr.endpoint.parse().map_err(user_error(BAD_CODE))?;

    let connection = transport::connect_pinned(
        &state.transport_cert,
        qr.transport_spki_hash,
        addr,
        &DialConfig::default(),
    )
    .await
    .map_err(user_error(UNREACHABLE))?;

    let (mut send_stream, mut recv_stream) = connection
        .open_bi()
        .await
        .map_err(user_error(UNREACHABLE))?;

    let responder = pairing::ResponderPairing::new(
        state.identity_signer.clone(),
        state.transport_cert.clone(),
        state.trust_store.clone(),
    );

    let trusted_peer = responder
        .complete_handshake(&qr, &mut send_stream, &mut recv_stream)
        .await
        .map_err(user_error(PAIRING_FAILED))?;

    if let Ok(mut allowed) = state.allowed_spki_hashes.write() {
        allowed.insert(trusted_peer.transport_spki_hash);
    }
    sessions::remember_peer_address(&state.trust_store, &trusted_peer.fingerprint, addr.ip());
    state.sessions.redial_now();

    // The pairing connection is dropped here; the session is dialed at the saved address.
    Ok(TrustedPeerDto {
        fingerprint: trusted_peer.fingerprint,
        display_name: shown_name(trusted_peer.display_name),
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
            .map_err(user_error("Couldn't load what this device can do."))?;

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
        _ => return Err(user_error(SAVE_FAILED)(format!("unknown grant {grant}"))),
    };

    state
        .permission_store
        .set_persisted_grant(
            &peer_fingerprint,
            CapabilityId(capability_id),
            1,
            parsed_grant,
        )
        .map_err(user_error(SAVE_FAILED))?;

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
        .map_err(user_error("Couldn't load this device."))?
        .ok_or_else(|| "This device isn't paired any more.".to_string())?;

    let addr: std::net::SocketAddr = endpoint.trim().parse().map_err(user_error(
        "Enter the address with its port, like 192.168.1.20:47470.",
    ))?;

    state
        .sessions
        .connect(&peer_fingerprint, peer.transport_spki_hash, addr)
        .await
        .map_err(user_error("Couldn't reach the device at that address."))?;

    // The connection is up either way; failing to remember the address only matters next time.
    if let Err(error) = state
        .trust_store
        .set_last_endpoint(&peer_fingerprint, &addr.to_string())
    {
        tracing::warn!("Couldn't save the address for {peer_fingerprint} ({error})");
    }
    Ok(())
}

#[tauri::command]
async fn disconnect_peer(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<(), String> {
    state.sessions.disconnect(&peer_fingerprint).await;
    Ok(())
}

#[tauri::command]
fn reconnect_peer(state: State<DesktopRuntimeState>, peer_fingerprint: String) {
    state.sessions.resume_auto_connect(&peer_fingerprint);
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
    if state.sessions.get(&peer_fingerprint).is_none() {
        return Err(NOT_CONNECTED.to_string());
    }

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

    // Waits out a dropped connection and sends the rest once it's back.
    let result = state
        .sessions
        .send_file(&peer_fingerprint, &path, transfer_id, Some(report))
        .await;
    remember(
        &state.history,
        history::Item {
            direction: Direction::Sent,
            kind: Kind::File,
            label: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            peer_fingerprint: peer_fingerprint.clone(),
            peer_name: peer_name(&state.trust_store, &peer_fingerprint),
            size: std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
            failed: result.is_err(),
            location: Some(path.to_string_lossy().into_owned()),
        },
    );
    result.map_err(user_error("Couldn't send the file."))
}

#[tauri::command]
async fn send_clipboard_text(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    text: String,
) -> Result<(), String> {
    push_text(
        &state.sessions,
        &state.trust_store,
        &state.history,
        peer_fingerprint,
        text,
    )
    .await
}

/// Text copied here while sync is on, as the window lists it.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncedTextDto {
    peer_id: String,
    peer_name: String,
    text: String,
    failed: bool,
}

/// Sends what was just copied here to every connected device.
fn send_copied_text(app: &AppHandle, text: String) {
    let state = app.state::<DesktopRuntimeState>();
    let (sessions, trust_store, history) = (
        state.sessions.clone(),
        state.trust_store.clone(),
        state.history.clone(),
    );
    let peers = trust_store.list_peers().unwrap_or_default();
    for peer in peers
        .into_iter()
        .filter(|p| sessions.get(&p.fingerprint).is_some())
    {
        let (app, sessions, trust_store, history, text) = (
            app.clone(),
            sessions.clone(),
            trust_store.clone(),
            history.clone(),
            text.clone(),
        );
        tauri::async_runtime::spawn(async move {
            let result = push_text(
                &sessions,
                &trust_store,
                &history,
                peer.fingerprint.clone(),
                text.clone(),
            )
            .await;
            let _ = app.emit(
                "clipboard-synced",
                SyncedTextDto {
                    peer_id: peer.fingerprint,
                    peer_name: peer.display_name,
                    text,
                    failed: result.is_err(),
                },
            );
        });
    }
}

/// Turns sending what's copied here on or off. The window says on start and on each change.
#[tauri::command]
fn set_clipboard_sync(state: State<DesktopRuntimeState>, enabled: bool) {
    state.clipboard.set_enabled(enabled);
}

/// Sends text to a connected device and saves it to history.
async fn push_text(
    sessions: &SessionRegistry,
    trust_store: &TrustStore,
    history: &HistoryStore,
    peer_fingerprint: String,
    text: String,
) -> Result<(), String> {
    let mux = sessions
        .get(&peer_fingerprint)
        .ok_or_else(|| NOT_CONNECTED.to_string())?;

    let mut caps = std::collections::HashSet::new();
    caps.insert(protocol::CapabilityId::CLIPBOARD);
    let query = capabilities::CapabilityQuery {
        capability: protocol::CapabilityId::CLIPBOARD,
        is_os_available: true,
        is_app_permitted: true,
        is_peer_authorized: true,
        negotiated_session_capabilities: caps,
    };

    let result = mux
        .send_clipboard_to_peer(
            clipboard::ClipboardFormat::TextPlain,
            text.clone().into_bytes(),
            &query,
        )
        .await;
    remember(
        history,
        history::Item {
            direction: Direction::Sent,
            kind: Kind::Text,
            size: text.len() as u64,
            label: text,
            peer_name: peer_name(trust_store, &peer_fingerprint),
            peer_fingerprint,
            failed: result.is_err(),
            location: None,
        },
    );
    result
        .map(|_| ())
        .map_err(user_error("Couldn't send the text."))
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
        .ok_or_else(|| NOT_CONNECTED.to_string())?;

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
        .map_err(user_error("Couldn't send the notification."))?;

    Ok(())
}

fn initialize_desktop_runtime(
    app_handle: &AppHandle,
    db_path: &Path,
    secrets_dir: &Path,
    download_dir: PathBuf,
    clipboard: ClipboardSync,
) -> Result<DesktopRuntimeState, Box<dyn std::error::Error>> {
    let trust_store = TrustStore::open(db_path)?;
    let permission_store = Arc::new(PermissionStore::open(db_path)?);
    let history = HistoryStore::open(db_path)?;

    let DeviceKeys {
        identity_signer,
        transport_cert,
    } = DeviceKeys::load_or_create(&secrets::device_key_store(secrets_dir)?)?;

    let peers = trust_store.list_peers()?;
    let initial_hashes: HashSet<[u8; 32]> = peers.iter().map(|p| p.transport_spki_hash).collect();
    let allowed_spki_hashes = Arc::new(std::sync::RwLock::new(initial_hashes));

    let local_fingerprint =
        identity::Fingerprint::from_verifying_key(&identity_signer.verifying_key()?).to_string();
    let pending_answers = PendingAnswers::default();
    let connected = Arc::new(tray::Connected::default());
    let mut handlers = session_handlers(
        download_dir,
        permission_store.clone(),
        trust_store.clone(),
        pending_answers.clone(),
        history.clone(),
        clipboard.clone(),
        app_handle,
    );
    handlers.on_device_info = Some(device_info_listener(
        app_handle.clone(),
        trust_store.clone(),
        connected.clone(),
    ));
    let (save_folder, incoming) = (handlers.save_folder.clone(), handlers.incoming.clone());
    let sessions = SessionRegistry::new(
        local_fingerprint,
        transport_cert.clone(),
        handlers,
        sessions::RegistryConfig::default(),
        Some(session_state_listener(
            app_handle.clone(),
            trust_store.clone(),
            connected,
        )),
    );

    let device_name = computer_name();

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
        pending_answers,
        history,
        save_folder,
        incoming,
        clipboard,
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
        // Opening Continue again shows the one already running, which may be in the tray.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            tray::show_window(app)
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![BACKGROUND_ARG]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_data = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let _ = std::fs::create_dir_all(&app_data);

            let db_path = app_data.join("continue_desktop.db");
            let secrets_dir = app_data.join("secrets");
            let _ = std::fs::create_dir_all(&secrets_dir);

            let _ = std::fs::remove_dir_all(pasted_dir(app.handle()));

            let download_dir = receiving::starting_folder(app.handle(), &app_data);
            let _ = std::fs::create_dir_all(&download_dir);

            let (clipboard, clipboard_watcher) = clipboard_sync::new();
            let runtime_state = initialize_desktop_runtime(
                app.handle(),
                &db_path,
                &secrets_dir,
                download_dir,
                clipboard,
            )
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
            let handle = app.handle().clone();
            clipboard_watcher.start(move |text| send_copied_text(&handle, text));

            if let Err(error) = tray::create(app.handle()) {
                tracing::warn!("No tray icon, so closing the window will quit: {error}");
            }
            start_at_login_by_default(app.handle(), &app_data);
            if !std::env::args().any(|arg| arg == BACKGROUND_ARG) {
                tray::show_window(app.handle());
            }
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
            answer_permission,
            pending_permission_questions,
            get_history,
            clear_history,
            open_received,
            open_link,
            set_clipboard_sync,
            receiving::list_incoming,
            receiving::cancel_incoming,
            receiving::get_save_folder,
            receiving::set_save_folder,
            get_autostart,
            set_autostart,
            save_pasted_file,
            connect_to_peer,
            disconnect_peer,
            reconnect_peer,
            send_file_to_peer,
            send_clipboard_text,
            send_notification
        ])
        // Closing the window keeps Continue in the tray, still receiving. Quit is in the tray menu.
        .on_window_event(|window, event| {
            let in_tray = tray::exists(window.app_handle());
            if let (tauri::WindowEvent::CloseRequested { api, .. }, true) = (event, in_tray) {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Continue desktop application");
}
