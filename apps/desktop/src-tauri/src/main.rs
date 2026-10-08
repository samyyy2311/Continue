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
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use capabilities::CapabilityQuery;
use clipboard::history::ClipboardHistoryStore;
use device::{ConnectError, Device, PairError, SendError, Stores};
use history::{Direction, HistoryStore, Kind};
use pairing::DeviceKeys;
use permissions::PermissionState;
use protocol::v1::{CatalogQuery, PcAction, PcActionRequest, RingRequest, ThumbnailRequest};
use protocol::CapabilityId;
use sessions::{PermissionDecision, SessionState};
use transfer::TransferError;

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
    /// Only while connected, and once the device has said.
    pub battery: Option<BatteryDto>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct BatteryDto {
    pub percent: u32,
    pub charging: bool,
}

/// The latest battery each device reported, by fingerprint.
type Batteries = Arc<Mutex<HashMap<String, BatteryDto>>>;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PeerPermissionDto {
    pub capability_id: u32,
    pub grant: &'static str,
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
    device: Device,
    /// The pairing code on show, waiting for a phone.
    active_pairing: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pending_answers: PendingAnswers,
    save_folder: sessions::SaveFolder,
    incoming: sessions::IncomingFiles,
    clipboard: ClipboardSync,
    batteries: Batteries,
    clipboard_history: Arc<ClipboardHistoryStore>,
    cloud_mounts: Arc<Mutex<HashMap<String, Arc<transfer::CloudFilesMount>>>>,
    active_handoffs: Arc<Mutex<HashMap<String, HandoffItemDto>>>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HandoffItemDto {
    pub handoff_id: String,
    pub peer_id: String,
    pub source_device_id: String,
    pub handoff_type: i32,
    pub title: String,
    pub uri: String,
    pub scroll_ratio: f32,
    pub cursor_position: u64,
    pub timestamp_ms: u64,
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
        .device
        .stores
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
        .device
        .stores
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
    if !file.starts_with(&folder) && !was_received(&state.device.stores.history, &file) {
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
    stores: &Stores,
    pending_answers: PendingAnswers,
    clipboard: ClipboardSync,
    clipboard_history: Arc<ClipboardHistoryStore>,
    active_handoffs: Arc<Mutex<HashMap<String, HandoffItemDto>>>,
    app_handle: &AppHandle,
) -> sessions::SessionCapabilityHandlers {
    let mut handlers = sessions::SessionCapabilityHandlers::new(download_dir)
        .with_this_device(sessions::ThisDevice::new(
            computer_name(),
            sessions::this_platform(),
        ))
        .with_permission_prompt(permission_prompt(
            app_handle.clone(),
            stores.clone(),
            pending_answers,
        ))
        .with_incoming(sessions::IncomingFiles::with_listener(receiving::listener(
            app_handle.clone(),
            stores.clone(),
        )));

    let (app, files) = (app_handle.clone(), stores.clone());
    handlers.on_file_received = Some(Arc::new(move |peer, file| {
        let location = file.path.to_string_lossy().into_owned();
        let name = files
            .received_file(peer, &file, Some(location.clone()))
            .peer_name;
        notify_if_away(
            &app,
            &format!("{} sent a file", shown_name(name.clone())),
            &file.file_name,
        );
        let _ = app.emit(
            "file-received",
            serde_json::json!({
                "peerId": peer,
                "peerName": name,
                "fileName": file.file_name,
                "path": location,
                "bytesReceived": file.bytes_received,
            }),
        );
    }));

    let (app, texts, history_store) = (app_handle.clone(), stores.clone(), clipboard_history);
    handlers.on_clipboard_received = Some(Arc::new(move |peer, update| {
        let text = String::from_utf8_lossy(&update.payload).into_owned();
        clipboard.write(text.clone());
        let name = texts.received_text(peer, &text).peer_name;
        let _ = history_store.record_clip(
            protocol::v1::ClipboardFormat::TextPlain,
            &text,
            &name,
            None::<[&str; 0]>,
        );
        notify_if_away(
            &app,
            &format!("Copied text from {}", shown_name(name.clone())),
            text.lines().next().unwrap_or_default(),
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

    let app = app_handle.clone();
    handlers.on_notification = Some(Arc::new(move |peer, body| match body {
        notifications::Body::Post(post) => {
            notify_if_away(&app, &post.title, &post.body);
            let _ = app.emit(
                "notification-posted",
                PhoneNotificationDto {
                    peer_id: peer.to_string(),
                    id: post.notification_id,
                    app_name: post.app_name,
                    title: post.title,
                    text: post.body,
                    posted_at: post.timestamp,
                    buttons: post
                        .actions
                        .into_iter()
                        .map(|action| NotificationButtonDto {
                            id: action.action_id,
                            label: action.label,
                            is_reply: action.is_reply,
                        })
                        .collect(),
                },
            );
        }
        notifications::Body::Dismiss(dismiss) => {
            let _ = app.emit("notification-removed", dismiss.notification_id);
        }
        // Only the phone carries out button presses.
        notifications::Body::Action(_) => {}
    }));

    let (app, handoffs_store) = (app_handle.clone(), active_handoffs.clone());
    handlers.on_handoff_received = Some(Arc::new(move |peer, item| {
        let dto = HandoffItemDto {
            handoff_id: item.handoff_id.clone(),
            peer_id: peer.to_string(),
            source_device_id: item.source_device_id,
            handoff_type: item.handoff_type,
            title: item.title,
            uri: item.uri,
            scroll_ratio: item.scroll_ratio,
            cursor_position: item.cursor_position,
            timestamp_ms: item.timestamp_ms,
        };
        handoffs_store.lock().insert(item.handoff_id.clone(), dto.clone());
        let _ = app.emit("handoff-received", dto);
    }));

    let (app, handoffs_store) = (app_handle.clone(), active_handoffs);
    handlers.on_handoff_dismissed = Some(Arc::new(move |_peer, handoff_id| {
        handoffs_store.lock().remove(&handoff_id);
        let _ = app.emit("handoff-dismissed", handoff_id);
    }));

    handlers
}

/// A notification from the phone, as the window shows it.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PhoneNotificationDto {
    peer_id: String,
    id: String,
    app_name: String,
    title: String,
    text: String,
    /// Unix time in milliseconds.
    posted_at: u64,
    buttons: Vec<NotificationButtonDto>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct NotificationButtonDto {
    id: String,
    label: String,
    is_reply: bool,
}

/// Saves the name a device sends when it connects, and shows it in the window and tray.
fn device_info_listener(
    app: AppHandle,
    stores: Stores,
    connected: Arc<tray::Connected>,
) -> sessions::OnReceived<sessions::PeerDevice> {
    Arc::new(move |peer, device| {
        if stores.rename_peer(peer, &device.name) {
            connected.rename(&app, peer, device.name);
            let _ = app.emit("peer-renamed", peer);
        }
    })
}

/// Keeps each device's latest battery for the window, and tells it when one changes.
fn device_status_listener(
    app: AppHandle,
    batteries: Batteries,
) -> sessions::OnReceived<protocol::v1::DeviceStatus> {
    Arc::new(move |peer, status| {
        let battery = BatteryDto {
            percent: status.battery_percent,
            charging: status.charging,
        };
        batteries.lock().insert(peer.to_string(), battery);
        let _ = app.emit("peer-status", peer);
    })
}

/// Asks in the window, which answers through `answer_permission`. The core stops waiting
/// after `PROMPT_TIMEOUT`, and so does the window.
fn permission_prompt(
    app_handle: AppHandle,
    stores: Stores,
    pending_answers: PendingAnswers,
) -> sessions::PermissionPrompt {
    let next_id = AtomicU64::new(0);
    Arc::new(move |request| {
        let (answer, decision) = tokio::sync::oneshot::channel();
        let question = PermissionQuestionDto {
            id: next_id.fetch_add(1, Ordering::Relaxed),
            peer_name: stores.peer_name(&request.peer),
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
    stores: Stores,
    connected: Arc<tray::Connected>,
) -> sessions::StateListener {
    Arc::new(move |peer, session_state| {
        let is_connected = session_state == SessionState::Connected;
        let name = shown_name(stores.peer_name(peer));
        connected.update(&app_handle, peer, name.clone(), is_connected);
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
                    "displayName": name,
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
    let trust = &state.device.stores.trust;
    let peers = trust
        .list_peers()
        .map_err(user_error("Couldn't load your devices."))?;

    let dtos = peers
        .into_iter()
        .map(|p| {
            let is_connected =
                state.device.sessions.state(&p.fingerprint) == SessionState::Connected;
            let endpoint = trust.last_endpoint(&p.fingerprint).ok().flatten();
            let battery = is_connected
                .then(|| state.batteries.lock().get(&p.fingerprint).copied())
                .flatten();
            TrustedPeerDto {
                fingerprint: p.fingerprint,
                display_name: shown_name(p.display_name),
                paired_at: p.paired_at,
                is_connected,
                endpoint,
                battery,
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
    state
        .device
        .forget(&fingerprint)
        .await
        .map_err(user_error("Couldn't forget this device. Try again."))
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
    let lan_ip = local_lan_ip()?;
    let server = state
        .device
        .start_pairing(0, |port| {
            std::net::SocketAddr::new(lan_ip, port).to_string()
        })
        .map_err(user_error(PAIRING_SETUP_FAILED))?;
    let code = server.code.clone();
    // Replacing or cancelling the attempt drops it, so it never reports into the window.
    let waiting = tokio::spawn(async move {
        let _ = match server.finish().await {
            Ok(peer) => app.emit("pairing-completed", paired_dto(peer)),
            Err(e) => app.emit("pairing-failed", user_error(PAIRING_FAILED)(e)),
        };
    });
    if let Some(previous) = state.active_pairing.lock().replace(waiting) {
        previous.abort();
    }
    Ok(code)
}

#[tauri::command]
fn cancel_pairing(state: State<DesktopRuntimeState>) {
    if let Some(waiting) = state.active_pairing.lock().take() {
        waiting.abort();
    }
}

#[tauri::command]
async fn pair_from_qr(
    state: State<'_, DesktopRuntimeState>,
    qr_payload: String,
) -> Result<TrustedPeerDto, String> {
    match state.device.pair_with_code(&qr_payload).await {
        Ok(peer) => Ok(paired_dto(peer)),
        Err(e @ PairError::BadCode(_)) => Err(user_error(BAD_CODE)(e)),
        Err(e @ PairError::Unreachable(_)) => Err(user_error(UNREACHABLE)(e)),
        Err(e @ PairError::Failed(_)) => Err(user_error(PAIRING_FAILED)(e)),
    }
}

/// A device just paired. It isn't connected yet: the session is dialed at its saved address.
fn paired_dto(peer: pairing::TrustedPeer) -> TrustedPeerDto {
    TrustedPeerDto {
        fingerprint: peer.fingerprint,
        display_name: shown_name(peer.display_name),
        paired_at: peer.paired_at,
        is_connected: false,
        endpoint: None,
        battery: None,
    }
}

#[tauri::command]
fn get_permissions(
    state: State<DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<Vec<PeerPermissionDto>, String> {
    [
        CapabilityId::FILE_TRANSFER,
        CapabilityId::CLIPBOARD,
        CapabilityId::NOTIFICATIONS,
    ]
    .into_iter()
    .map(|capability| {
        let grant = state
            .device
            .stores
            .permissions
            .query_state(&peer_fingerprint, capability)
            .map_err(user_error("Couldn't load what this device can do."))?;
        Ok(PeerPermissionDto {
            capability_id: capability.raw(),
            grant: grant.as_str(),
        })
    })
    .collect()
}

#[tauri::command]
fn set_permission(
    state: State<DesktopRuntimeState>,
    peer_fingerprint: String,
    capability_id: u32,
    grant: String,
) -> Result<(), String> {
    let permission = PermissionState::parse(&grant)
        .ok_or_else(|| user_error(SAVE_FAILED)(format!("unknown grant {grant}")))?;
    state
        .device
        .stores
        .permissions
        .set_state(&peer_fingerprint, CapabilityId(capability_id), permission)
        .map_err(user_error(SAVE_FAILED))
}

#[tauri::command]
async fn connect_to_peer(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    endpoint: String,
) -> Result<(), String> {
    let addr: std::net::SocketAddr = endpoint.trim().parse().map_err(user_error(
        "Enter the address with its port, like 192.168.1.20:47470.",
    ))?;
    match state.device.connect(&peer_fingerprint, addr).await {
        Ok(()) => Ok(()),
        Err(ConnectError::NotPaired) => Err("This device isn't paired any more.".to_string()),
        Err(e @ ConnectError::Store(_)) => Err(user_error("Couldn't load this device.")(e)),
        Err(e @ ConnectError::Unreachable(_)) => {
            Err(user_error("Couldn't reach the device at that address.")(e))
        }
    }
}

#[tauri::command]
async fn disconnect_peer(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<(), String> {
    state.device.sessions.disconnect(&peer_fingerprint).await;
    Ok(())
}

#[tauri::command]
fn reconnect_peer(state: State<DesktopRuntimeState>, peer_fingerprint: String) {
    state.device.sessions.resume_auto_connect(&peer_fingerprint);
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
    let path = PathBuf::from(file_path);
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

    let location = Some(path.to_string_lossy().into_owned());
    match state
        .device
        .send_file(&peer_fingerprint, &path, location, Some(report))
        .await
    {
        Ok(sent) => Ok(sent),
        Err(TransferError::NotConnected) => Err(NOT_CONNECTED.to_string()),
        Err(e) => Err(user_error("Couldn't send the file.")(e)),
    }
}

#[tauri::command]
async fn send_clipboard_text(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    text: String,
) -> Result<(), String> {
    push_text(&state.device, &peer_fingerprint, text).await
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
    let _ = state.clipboard_history.record_clip(
        protocol::v1::ClipboardFormat::TextPlain,
        &text,
        "This PC",
        None::<[&str; 0]>,
    );
    let device = state.device.clone();
    let peers = device.stores.trust.list_peers().unwrap_or_default();
    for peer in peers
        .into_iter()
        .filter(|p| device.sessions.get(&p.fingerprint).is_some())
    {
        let (app, device, text) = (app.clone(), device.clone(), text.clone());
        tauri::async_runtime::spawn(async move {
            let result = push_text(&device, &peer.fingerprint, text.clone()).await;
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

/// Presses a button on one of the phone's notifications, with `reply` for one that takes text.
#[tauri::command]
async fn press_notification_button(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    notification_id: String,
    button_id: String,
    reply: String,
) -> Result<(), String> {
    let action = notifications::NotificationActionInvoke {
        notification_id,
        action_id: button_id,
        reply_text: reply,
    };
    send_notification(
        &state,
        &peer_fingerprint,
        notifications::Body::Action(action),
    )
    .await
}

/// Clears one of the phone's notifications, there as well as here.
#[tauri::command]
async fn dismiss_notification(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    notification_id: String,
) -> Result<(), String> {
    let dismiss = notifications::NotificationDismiss {
        notification_id,
        package_name: String::new(),
    };
    send_notification(
        &state,
        &peer_fingerprint,
        notifications::Body::Dismiss(dismiss),
    )
    .await
}

async fn send_notification(
    state: &DesktopRuntimeState,
    peer: &str,
    body: notifications::Body,
) -> Result<(), String> {
    match state.device.send_notification(peer, body).await {
        Ok(()) => Ok(()),
        Err(SendError::NotConnected) => Err(NOT_CONNECTED.to_string()),
        Err(e) => Err(user_error("Couldn't reach your phone. Try again.")(e)),
    }
}

/// Sends text to a connected device and saves it to history.
async fn push_text(device: &Device, peer: &str, text: String) -> Result<(), String> {
    match device.send_text(peer, text).await {
        Ok(()) => Ok(()),
        Err(SendError::NotConnected) => Err(NOT_CONNECTED.to_string()),
        Err(e) => Err(user_error("Couldn't send the text.")(e)),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItemDto {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub timestamp: u64,
    pub mime_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogResponseDto {
    pub items: Vec<CatalogItemDto>,
    pub total_count: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardHistoryEntryDto {
    pub id: i64,
    pub timestamp_ms: u64,
    pub content: String,
    pub is_pinned: bool,
    pub origin_device: String,
}

#[tauri::command]
async fn ring_peer(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    active: bool,
) -> Result<bool, String> {
    let mux = state
        .device
        .sessions
        .get(&peer_fingerprint)
        .ok_or_else(|| NOT_CONNECTED.to_string())?;
    let query = CapabilityQuery::negotiated(CapabilityId::RING_DEVICE, true);
    let request = RingRequest {
        active,
        duration_secs: 30,
        force_max_volume: true,
    };
    let ack = mux
        .trigger_ring_on_peer(request, &query)
        .await
        .map_err(user_error("Couldn't ring device."))?;
    Ok(ack.is_ringing)
}

#[tauri::command]
async fn send_pc_action(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    action: i32,
) -> Result<bool, String> {
    let pc_action = match action {
        0 => PcAction::LockWorkstation,
        1 => PcAction::SleepSystem,
        2 => PcAction::ShutdownSystem,
        3 => PcAction::ToggleMute,
        _ => return Err("Invalid PC action".to_string()),
    };
    let mux = state
        .device
        .sessions
        .get(&peer_fingerprint)
        .ok_or_else(|| NOT_CONNECTED.to_string())?;
    let query = CapabilityQuery::negotiated(CapabilityId::PC_CONTROL, true);
    let request = PcActionRequest {
        action: pc_action as i32,
        force: false,
    };
    let resp = mux
        .send_pc_action_to_peer(request, &query)
        .await
        .map_err(user_error("Couldn't execute PC action."))?;
    Ok(resp.success)
}

#[tauri::command]
async fn query_file_catalog(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    category: i32,
    limit: u32,
    offset: u32,
) -> Result<CatalogResponseDto, String> {
    let mux = state
        .device
        .sessions
        .get(&peer_fingerprint)
        .ok_or_else(|| NOT_CONNECTED.to_string())?;
    let query = CapabilityQuery::negotiated(CapabilityId::FILE_CATALOG, true);
    let req = CatalogQuery {
        category,
        limit,
        offset,
    };
    let resp = mux
        .query_file_catalog_from_peer(req, &query)
        .await
        .map_err(user_error("Couldn't browse files on device."))?;
    Ok(CatalogResponseDto {
        items: resp
            .items
            .into_iter()
            .map(|item| CatalogItemDto {
                id: item.item_id,
                name: item.file_name,
                size_bytes: item.size_bytes,
                timestamp: item.timestamp,
                mime_type: item.mime_type,
            })
            .collect(),
        total_count: resp.total_count,
    })
}

#[tauri::command]
async fn get_catalog_thumbnail(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    item_id: String,
    max_edge: u32,
) -> Result<String, String> {
    let mux = state
        .device
        .sessions
        .get(&peer_fingerprint)
        .ok_or_else(|| NOT_CONNECTED.to_string())?;
    let query = CapabilityQuery::negotiated(CapabilityId::FILE_CATALOG, true);
    let req = ThumbnailRequest {
        item_id,
        max_dimension: max_edge,
    };
    let resp = mux
        .request_thumbnail_from_peer(req, &query)
        .await
        .map_err(user_error("Couldn't load thumbnail."))?;
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    let b64 = BASE64.encode(&resp.image_data);
    let mime = if resp.mime_type.is_empty() {
        "image/jpeg"
    } else {
        &resp.mime_type
    };
    Ok(format!("data:{mime};base64,{b64}"))
}

#[tauri::command]
async fn mount_cloud_files(
    app: AppHandle,
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<String, String> {
    let peer_name = state.device.stores.peer_name(&peer_fingerprint);
    let base_dir = std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|_| app.path().home_dir())
        .unwrap_or_else(|_| PathBuf::from("."));
    let mount_path = base_dir
        .join("Continue")
        .join(shown_name(peer_name.clone()));
    let _ = std::fs::create_dir_all(&mount_path);

    let config = transfer::CloudFilesConfig::new(
        &mount_path,
        "Continue",
        shown_name(peer_name),
        &peer_fingerprint,
    );

    let mount = Arc::new(transfer::CloudFilesMount::new(config));
    if transfer::CloudFilesMount::is_supported() {
        let _ = mount.register();

        if let Some(mux) = state.device.sessions.get(&peer_fingerprint) {
            let query = CapabilityQuery::negotiated(CapabilityId::FILE_CATALOG, true);
            let req = CatalogQuery {
                category: 0,
                limit: 100,
                offset: 0,
            };
            if let Ok(catalog) = mux.query_file_catalog_from_peer(req, &query).await {
                let entries: Vec<transfer::CloudFilesEntry> = catalog
                    .items
                    .into_iter()
                    .map(|item| {
                        transfer::CloudFilesEntry::file(
                            item.file_name,
                            item.size_bytes,
                            item.timestamp,
                            item.item_id.into_bytes(),
                        )
                    })
                    .collect();
                let _ = mount.create_placeholders(&entries);
            }
        }
    }

    state.cloud_mounts.lock().insert(peer_fingerprint, mount);
    let path_str = mount_path.to_string_lossy().into_owned();
    let _ = app.opener().open_path(&path_str, None::<&str>);
    Ok(path_str)
}

#[tauri::command]
fn open_cloud_files_folder(
    app: AppHandle,
    state: State<DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<(), String> {
    let peer_name = state.device.stores.peer_name(&peer_fingerprint);
    let base_dir = std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|_| app.path().home_dir())
        .unwrap_or_else(|_| PathBuf::from("."));
    let mount_path = base_dir.join("Continue").join(shown_name(peer_name));
    app.opener()
        .open_path(mount_path.to_string_lossy(), None::<&str>)
        .map_err(user_error("Couldn't open folder."))
}

#[tauri::command]
fn get_clipboard_history(
    state: State<DesktopRuntimeState>,
    limit: u32,
) -> Result<Vec<ClipboardHistoryEntryDto>, String> {
    let clips = state
        .clipboard_history
        .list_clips(limit.max(1))
        .map_err(user_error("Couldn't load clipboard history."))?;
    Ok(clips
        .into_iter()
        .map(|c| ClipboardHistoryEntryDto {
            id: c.id,
            timestamp_ms: c.timestamp_ms,
            content: c.content,
            is_pinned: c.is_pinned,
            origin_device: c.origin_device,
        })
        .collect())
}

#[tauri::command]
fn pin_clipboard_clip(
    state: State<DesktopRuntimeState>,
    id: i64,
    pinned: bool,
) -> Result<bool, String> {
    state
        .clipboard_history
        .pin_clip(id, pinned)
        .map_err(user_error("Couldn't pin clip."))
}

#[tauri::command]
fn delete_clipboard_clip(state: State<DesktopRuntimeState>, id: i64) -> Result<bool, String> {
    state
        .clipboard_history
        .delete_clip(id)
        .map_err(user_error("Couldn't delete clip."))
}

#[tauri::command]
fn clear_clipboard_history(state: State<DesktopRuntimeState>) -> Result<usize, String> {
    state
        .clipboard_history
        .clear_unpinned()
        .map_err(user_error("Couldn't clear clipboard history."))
}

#[tauri::command]
fn get_active_handoffs(state: State<DesktopRuntimeState>) -> Vec<HandoffItemDto> {
    state.active_handoffs.lock().values().cloned().collect()
}

#[tauri::command]
async fn broadcast_handoff(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    title: String,
    uri: String,
    handoff_type: i32,
    scroll_ratio: f32,
) -> Result<bool, String> {
    let mux = state
        .device
        .sessions
        .get(&peer_fingerprint)
        .ok_or_else(|| NOT_CONNECTED.to_string())?;
    let query = CapabilityQuery::negotiated(CapabilityId::HANDOFF, true);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let handoff_id = format!("h-{now_ms}");
    let item = protocol::v1::HandoffItem {
        handoff_id,
        source_device_id: computer_name(),
        handoff_type,
        title,
        uri,
        scroll_ratio,
        cursor_position: 0,
        timestamp_ms: now_ms,
        extra_payload: Vec::new(),
    };
    let ack = mux
        .broadcast_handoff_to_peer(item, &query)
        .await
        .map_err(user_error("Couldn't send handoff to device."))?;
    Ok(ack.success)
}

#[tauri::command]
async fn dismiss_handoff(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    handoff_id: String,
) -> Result<bool, String> {
    state.active_handoffs.lock().remove(&handoff_id);
    if let Some(mux) = state.device.sessions.get(&peer_fingerprint) {
        let query = CapabilityQuery::negotiated(CapabilityId::HANDOFF, true);
        let _ = mux.dismiss_handoff_on_peer(handoff_id, &query).await;
    }
    Ok(true)
}

#[tauri::command]
async fn open_handoff(
    app: AppHandle,
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    handoff_id: String,
    uri: String,
) -> Result<(), String> {
    state.active_handoffs.lock().remove(&handoff_id);
    let _ = app.opener().open_url(&uri, None::<&str>);
    if let Some(mux) = state.device.sessions.get(&peer_fingerprint) {
        let query = CapabilityQuery::negotiated(CapabilityId::HANDOFF, true);
        let _ = mux.dismiss_handoff_on_peer(handoff_id, &query).await;
    }
    Ok(())
}

fn initialize_desktop_runtime(
    app_handle: &AppHandle,
    db_path: &Path,
    secrets_dir: &Path,
    download_dir: PathBuf,
    clipboard: ClipboardSync,
) -> Result<DesktopRuntimeState, Box<dyn std::error::Error>> {
    let stores = Stores::open(db_path)?;
    let keys = DeviceKeys::load_or_create(&secrets::device_key_store(secrets_dir)?)?;
    let clipboard_history = Arc::new(ClipboardHistoryStore::open(
        db_path.with_file_name("continue_clipboard.db"),
    )?);
    let cloud_mounts = Arc::new(Mutex::new(HashMap::new()));
    let active_handoffs = Arc::new(Mutex::new(HashMap::new()));

    let pending_answers = PendingAnswers::default();
    let connected = Arc::new(tray::Connected::default());
    let mut handlers = session_handlers(
        download_dir,
        &stores,
        pending_answers.clone(),
        clipboard.clone(),
        clipboard_history.clone(),
        active_handoffs.clone(),
        app_handle,
    );
    handlers.on_device_info = Some(device_info_listener(
        app_handle.clone(),
        stores.clone(),
        connected.clone(),
    ));
    let batteries = Batteries::default();
    handlers.on_device_status = Some(device_status_listener(
        app_handle.clone(),
        batteries.clone(),
    ));
    let (save_folder, incoming) = (handlers.save_folder.clone(), handlers.incoming.clone());
    let on_state_change = session_state_listener(app_handle.clone(), stores.clone(), connected);
    let device = Device::new(stores, keys, handlers, Some(on_state_change))?;

    Ok(DesktopRuntimeState {
        device_name: computer_name(),
        device,
        active_pairing: Mutex::new(None),
        pending_answers,
        save_folder,
        incoming,
        clipboard,
        batteries,
        clipboard_history,
        cloud_mounts,
        active_handoffs,
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
            let device = runtime_state.device.clone();
            tauri::async_runtime::spawn(async move {
                let listener = device
                    .listen()
                    .and_then(|endpoint| Ok(endpoint.local_addr()?.port()));
                match listener {
                    Ok(port) => {
                        tracing::info!("Listening for paired devices on port {port}");
                        device.discover(port, 1);
                    }
                    Err(error) => tracing::error!("Could not listen for paired devices: {error}"),
                }
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
            press_notification_button,
            dismiss_notification,
            ring_peer,
            send_pc_action,
            query_file_catalog,
            get_catalog_thumbnail,
            mount_cloud_files,
            open_cloud_files_folder,
            get_clipboard_history,
            pin_clipboard_clip,
            delete_clipboard_clip,
            clear_clipboard_history,
            get_active_handoffs,
            broadcast_handoff,
            dismiss_handoff,
            open_handoff
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
