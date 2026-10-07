// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod call_camera;
mod clipboard_sync;
mod drop_folder;
mod look;
mod pointer;
mod proximity;
mod receiving;
mod secrets;
mod send_to;
mod tray;
mod video;

use clipboard_sync::{Clip, ClipboardSync};
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

use device::{ConnectError, Device, PairError, SendError, Stores};
use history::{Direction, HistoryStore, Kind};
use pairing::DeviceKeys;
use protocol::v1::{
    call_action, calls_message, files_message, media_message, messages_message, photos_message,
    CallAction, CallsReply, Conversation, FileEntry, FilesReply, GetFile, ListConversations,
    ListFolder, ListPhotos, MediaCommand, MediaReply, MessagesReply, PhotosReply, ReadConversation,
    SendPhoto, SendText, TextMessage,
};
use protocol::CapabilityId;
use sessions::SessionState;
use transfer::TransferError;

/// Logs what actually went wrong and gives the UI a sentence a person can act on.
fn user_error<E: std::fmt::Display>(message: &'static str) -> impl FnOnce(E) -> String {
    move |error| {
        tracing::warn!("{message} ({error})");
        message.to_string()
    }
}

/// Warns once per discharge when a phone reports this or less.
const LOW_BATTERY_PERCENT: u32 = 15;

pub(crate) const NOT_CONNECTED: &str = "That device isn't connected.";
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
    pub status: Option<StatusDto>,
    /// The main colour of the device's wallpaper, as #rrggbb, once it has said.
    pub wallpaper_color: Option<String>,
    /// JPEG data URL of the wallpaper, when the device could read it.
    pub wallpaper: Option<String>,
    /// The same four words show on the device, to check it's the one that was paired.
    pub words: String,
}

fn pairing_words(device: &Device, peer: &str) -> String {
    pairing::pairing_words(&device.fingerprint, peer).join(" ")
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatusDto {
    pub percent: u32,
    pub charging: bool,
    pub cell_bars: Option<u32>,
    pub wifi_bars: Option<u32>,
    pub carrier: String,
    pub network: String,
}

/// Latest report from each device, by fingerprint.
#[derive(Default, Clone)]
struct PeerReport {
    status: Option<StatusDto>,
    /// Set once the low-battery alert is shown, until the phone charges again.
    warned_low: bool,
    /// Set once the fully-charged alert is shown, until the phone comes off charge.
    warned_full: bool,
    wallpaper_color: Option<u32>,
    wallpaper: Option<String>,
}

type PeerReports = Arc<Mutex<HashMap<String, PeerReport>>>;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PeerPermissionDto {
    pub capability_id: u32,
    pub allowed: bool,
}

pub struct DesktopRuntimeState {
    device_name: String,
    pub(crate) device: Device,
    /// The pairing code on show, waiting for a phone.
    active_pairing: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// A phone that paired and waits for the person to compare codes.
    pending_pair: Mutex<Option<device::PendingPair>>,
    save_folder: sessions::SaveFolder,
    incoming: sessions::IncomingFiles,
    clipboard: ClipboardSync,
    reports: PeerReports,
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
            kind: kind_name(entry.item.kind),
            label: entry.item.label,
            peer_id: entry.item.peer_fingerprint,
            peer_name: entry.item.peer_name,
            size: entry.item.size,
            failed: entry.item.failed,
            location: entry.item.location,
        })
        .collect())
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::File => "file",
        Kind::Text => "text",
    }
}

/// Text or a file kept for a device that isn't connected.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitingDto {
    pub id: i64,
    pub peer_id: String,
    pub peer_name: String,
    pub kind: &'static str,
    /// The text, or the file's path.
    pub content: String,
}

#[tauri::command]
fn get_waiting(state: State<DesktopRuntimeState>) -> Result<Vec<WaitingDto>, String> {
    let stores = &state.device.stores;
    let waiting = stores
        .history
        .waiting(None)
        .map_err(user_error("Couldn't load what's waiting to send."))?;
    Ok(waiting
        .into_iter()
        .map(|item| WaitingDto {
            id: item.id,
            peer_name: shown_name(stores.peer_name(&item.peer_fingerprint)),
            peer_id: item.peer_fingerprint,
            kind: kind_name(item.kind),
            content: item.content,
        })
        .collect())
}

/// Keeps text or a file for a device that isn't connected; it goes when the device connects.
#[tauri::command]
fn send_later(
    state: State<DesktopRuntimeState>,
    peer_fingerprint: String,
    is_text: bool,
    content: String,
) -> Result<i64, String> {
    let kind = if is_text { Kind::Text } else { Kind::File };
    state
        .device
        .send_later(&peer_fingerprint, kind, &content)
        .map_err(user_error(SAVE_FAILED))
}

#[tauri::command]
fn cancel_waiting(state: State<DesktopRuntimeState>, id: i64) -> Result<(), String> {
    state
        .device
        .stores
        .history
        .remove_waiting(id)
        .map(drop)
        .map_err(user_error(SAVE_FAILED))
}

/// A pinned clip, on every paired device.
#[derive(Serialize)]
pub struct SnippetDto {
    id: String,
    text: String,
}

impl From<history::Snippet> for SnippetDto {
    fn from(snippet: history::Snippet) -> Self {
        Self {
            id: snippet.id,
            text: snippet.text,
        }
    }
}

#[tauri::command]
fn get_snippets(state: State<DesktopRuntimeState>) -> Result<Vec<SnippetDto>, String> {
    let snippets = state
        .device
        .stores
        .history
        .snippets(false)
        .map_err(user_error("Couldn't load your pinned text."))?;
    Ok(snippets.into_iter().map(Into::into).collect())
}

#[tauri::command]
async fn pin_snippet(
    state: State<'_, DesktopRuntimeState>,
    text: String,
) -> Result<SnippetDto, String> {
    let snippet = state
        .device
        .pin(&text)
        .await
        .map_err(user_error(SAVE_FAILED))?;
    Ok(snippet.into())
}

#[tauri::command]
async fn unpin_snippet(state: State<'_, DesktopRuntimeState>, id: String) -> Result<(), String> {
    state
        .device
        .unpin(&id)
        .await
        .map_err(user_error(SAVE_FAILED))
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
    let file = received_file(&state, &path)?;
    let opener = app.opener();
    let result = if reveal {
        opener.reveal_item_in_dir(&file)
    } else {
        opener.open_path(file.to_string_lossy(), None::<&str>)
    };
    result.map_err(user_error("Couldn't open that file."))
}

/// `path` as a file this computer received. Anything else is refused, so the window can't be
/// used to open or read arbitrary files.
fn received_file(state: &DesktopRuntimeState, path: &str) -> Result<PathBuf, String> {
    const MISSING: &str = "Couldn't find that file. It may have been moved or deleted.";
    let file = std::fs::canonicalize(path).map_err(user_error(MISSING))?;
    let folder = std::fs::canonicalize(state.save_folder.get()).map_err(user_error(MISSING))?;
    // Files from before the folder was changed are still in history, so they count too.
    if !file.starts_with(&folder) && !was_received(&state.device.stores.history, &file) {
        return Err(MISSING.to_string());
    }
    Ok(file)
}

/// JPEG thumbnail as a data URL.
#[tauri::command]
async fn thumbnail(state: State<'_, DesktopRuntimeState>, path: String) -> Result<String, String> {
    let file = received_file(&state, &path)?;
    let jpeg = tauri::async_runtime::spawn_blocking(move || {
        let image = image::open(file).ok()?;
        look::jpeg(&image.thumbnail(96, 96))
    })
    .await
    .ok()
    .flatten()
    .ok_or_else(|| "Couldn't read that image.".to_string())?;
    Ok(jpeg_url(&jpeg))
}

fn jpeg_url(jpeg: &[u8]) -> String {
    use base64::Engine as _;
    format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(jpeg)
    )
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
/// There while connections are paused, so a pause lasts through a restart.
fn paused_marker(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|dir| dir.join("paused"))
}

pub(crate) fn set_paused(app: &AppHandle, paused: bool) {
    if let Some(marker) = paused_marker(app) {
        let saved = if paused {
            std::fs::write(marker, b"")
        } else {
            std::fs::remove_file(marker)
        };
        if let Err(error) = saved {
            tracing::warn!("Couldn't save whether connections are paused: {error}");
        }
    }
    let device = app.state::<DesktopRuntimeState>().device.clone();
    tauri::async_runtime::spawn(async move { device.sessions.set_paused(paused).await });
}

/// WebView2 acts on browser shortcuts itself: Ctrl+R reloads the window, Ctrl+P prints it and
/// Alt+Left goes back. Editing keys like copy and paste keep working.
#[cfg(windows)]
fn without_browser_keys(window: &tauri::WebviewWindow) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
    use windows::core::Interface;

    let _ = window.with_webview(|webview| unsafe {
        let settings = webview
            .controller()
            .CoreWebView2()
            .and_then(|core| core.Settings())
            .and_then(|settings| settings.cast::<ICoreWebView2Settings3>());
        if let Ok(settings) = settings {
            let _ = settings.SetAreBrowserAcceleratorKeysEnabled(false);
        }
    });
}

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

/// Where pasted files wait to be sent. Emptied on each start, except for files still
/// waiting for a device to connect.
fn pasted_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_cache_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("continue"))
        .join("pasted")
}

fn remove_old_pastes(app: &AppHandle, device: &Device) {
    let waiting: std::collections::HashSet<PathBuf> = device
        .stores
        .history
        .waiting(None)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| Path::new(&item.content).parent().map(Path::to_path_buf))
        .collect();
    let Ok(folders) = std::fs::read_dir(pasted_dir(app)) else {
        return;
    };
    for folder in folders.flatten().map(|entry| entry.path()) {
        if !waiting.contains(&folder) {
            let _ = std::fs::remove_dir_all(folder);
        }
    }
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
    // A folder per paste keeps the original name without clashing, including with folders
    // kept from earlier runs.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let folder = pasted_dir(&app).join(format!("{now}-{n}"));
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
    clipboard: ClipboardSync,
    app_handle: &AppHandle,
) -> sessions::SessionCapabilityHandlers {
    let mut handlers = sessions::SessionCapabilityHandlers::new(download_dir)
        .with_this_device(sessions::ThisDevice::new(
            computer_name(),
            sessions::this_platform(),
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

    let (app, texts) = (app_handle.clone(), stores.clone());
    handlers.on_clipboard_received = Some(Arc::new(move |peer, update| {
        if update.format() == protocol::v1::ClipboardFormat::ImagePng {
            clipboard.write(Clip::Image(update.payload));
            let name = shown_name(texts.peer_name(peer));
            notify_if_away(&app, &format!("Copied an image from {name}"), "");
            return;
        }
        let text = String::from_utf8_lossy(&update.payload).into_owned();
        clipboard.write(Clip::Text(text.clone()));
        let name = texts.received_text(peer, &text).peer_name;
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

    let (app, names) = (app_handle.clone(), stores.clone());
    handlers.on_photo_taken = Some(Arc::new(move |peer, photo| {
        let name = shown_name(names.peer_name(peer));
        notify_if_away(&app, &format!("New photo on {name}"), &photo.name);
        let _ = app.emit(
            "photo-taken",
            serde_json::json!({ "peerId": peer, "photo": PhotoDto::from(photo) }),
        );
    }));

    let app = app_handle.clone();
    handlers.on_now_playing = Some(Arc::new(move |peer, playing| {
        let art = (!playing.art.is_empty()).then(|| jpeg_url(&playing.art));
        let _ = app.emit(
            "now-playing",
            serde_json::json!({
                "peerId": peer,
                "title": playing.title,
                "artist": playing.artist,
                "app": playing.app,
                "playing": playing.playing,
                "durationMs": playing.duration_ms,
                "positionMs": playing.position_ms,
                "art": art,
            }),
        );
    }));

    let (app, names) = (app_handle.clone(), stores.clone());
    handlers.on_call = Some(Arc::new(move |peer, call| {
        let state = call.state();
        let caller = if call.name.is_empty() {
            call.number.clone()
        } else {
            call.name.clone()
        };
        if state == protocol::v1::call::State::Ringing {
            let name = shown_name(names.peer_name(peer));
            notify_if_away(&app, &format!("Call on {name}"), &caller);
        }
        let _ = app.emit(
            "phone-call",
            serde_json::json!({
                "peerId": peer,
                "state": state.as_str_name().to_lowercase(),
                "number": call.number,
                "name": call.name,
            }),
        );
    }));

    let app = app_handle.clone();
    handlers.on_messages_changed = Some(Arc::new(move |peer, ()| {
        let _ = app.emit("messages-changed", peer);
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
                    package_name: post.package_name,
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
        // Only the phone carries out button presses and mutes.
        notifications::Body::Action(_) | notifications::Body::Mute(_) => {}
    }));

    handlers
}

/// A notification from the phone, as the window shows it.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PhoneNotificationDto {
    peer_id: String,
    id: String,
    package_name: String,
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

fn device_status_listener(
    app: AppHandle,
    stores: Stores,
    reports: PeerReports,
) -> sessions::OnReceived<protocol::v1::DeviceStatus> {
    Arc::new(move |peer, status| {
        let mut reports = reports.lock();
        let report = reports.entry(peer.to_string()).or_default();
        let low = !status.charging && status.battery_percent <= LOW_BATTERY_PERCENT;
        if low && !report.warned_low {
            let name = shown_name(stores.peer_name(peer));
            let percent = status.battery_percent;
            notify_if_away(
                &app,
                &format!("{name} battery is low"),
                &format!("{percent}% left"),
            );
        }
        report.warned_low = low;
        let full = status.charging && status.battery_percent >= 100;
        if full && !report.warned_full {
            let name = shown_name(stores.peer_name(peer));
            notify_if_away(
                &app,
                &format!("{name} is fully charged"),
                "You can unplug it.",
            );
        }
        report.warned_full = full;
        report.status = Some(StatusDto {
            percent: status.battery_percent,
            charging: status.charging,
            cell_bars: status.cell_bars,
            wifi_bars: status.wifi_bars,
            carrier: status.carrier,
            network: status.network,
        });
        drop(reports);
        let _ = app.emit("peer-status", peer);
    })
}

fn device_look_listener(
    app: AppHandle,
    reports: PeerReports,
) -> sessions::OnReceived<protocol::v1::DeviceLook> {
    Arc::new(move |peer, look| {
        let mut reports = reports.lock();
        let report = reports.entry(peer.to_string()).or_default();
        report.wallpaper_color = (look.wallpaper_color != 0).then_some(look.wallpaper_color);
        report.wallpaper = (!look.wallpaper.is_empty()).then(|| jpeg_url(&look.wallpaper));
        drop(reports);
        let _ = app.emit("peer-status", peer);
    })
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
            send_waiting(&app_handle, peer);
        }
    })
}

/// Catches a device that just connected up on what changed while it was away.
fn send_waiting(app: &AppHandle, peer: &str) {
    if let Some(clip) = app.state::<CopiesWaiting>().0.lock().remove(peer) {
        send_copy(app, peer.to_string(), clip);
    }
    let device = app.state::<DesktopRuntimeState>().device.clone();
    let (app, peer) = (app.clone(), peer.to_string());
    tauri::async_runtime::spawn(async move {
        device.share_snippets(&peer).await;
        device
            .send_waiting(&peer, |item, sent| {
                let _ = app.emit(
                    "waiting-sent",
                    serde_json::json!({ "id": item.id, "sent": sent }),
                );
            })
            .await;
    });
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
            let report = state
                .reports
                .lock()
                .get(&p.fingerprint)
                .cloned()
                .unwrap_or_default();
            let status = report.status.filter(|_| is_connected);
            let wallpaper_color = report.wallpaper_color.map(|color| format!("#{color:06x}"));
            TrustedPeerDto {
                words: pairing_words(&state.device, &p.fingerprint),
                fingerprint: p.fingerprint,
                display_name: shown_name(p.display_name),
                paired_at: p.paired_at,
                is_connected,
                endpoint,
                status,
                wallpaper_color,
                wallpaper: report.wallpaper,
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
    let mut port = 0;
    let server = state
        .device
        .start_pairing(0, |bound| {
            port = bound;
            std::net::SocketAddr::new(lan_ip, bound).to_string()
        })
        .map_err(user_error(PAIRING_SETUP_FAILED))?;
    let code = server.code.clone();
    // Lets a phone nearby pick this computer from a list while the code is showing.
    let nearby = discovery::PairingAdvertiser::start(&state.device_name, &code, port)
        .map_err(|error| tracing::warn!("Not shown to phones nearby: {error}"))
        .ok();
    // Replacing or cancelling the attempt drops it, so it never reports into the window.
    let waiting = tokio::spawn(async move {
        let result = server.finish().await;
        drop(nearby);
        let _ = match result {
            Ok(pending) => {
                let code = pending.code().to_string();
                *app.state::<DesktopRuntimeState>().pending_pair.lock() = Some(pending);
                app.emit("pairing-check", code)
            }
            Err(e) => app.emit("pairing-failed", user_error(PAIRING_FAILED)(e)),
        };
    });
    if let Some(previous) = state.active_pairing.lock().replace(waiting) {
        previous.abort();
    }
    Ok(code)
}

/// Trusts the phone that paired once the person has seen the same code on both screens, or
/// forgets it if not.
#[tauri::command]
fn confirm_pairing(
    app: AppHandle,
    state: State<DesktopRuntimeState>,
    accept: bool,
) -> Result<(), String> {
    let Some(pending) = state.pending_pair.lock().take() else {
        return Err(PAIRING_FAILED.to_string());
    };
    if accept {
        let peer = pending.accept().map_err(user_error(PAIRING_FAILED))?;
        let _ = app.emit("pairing-completed", paired_dto(&state.device, peer));
    }
    Ok(())
}

#[tauri::command]
fn cancel_pairing(state: State<DesktopRuntimeState>) {
    state.pending_pair.lock().take();
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
        Ok(peer) => Ok(paired_dto(&state.device, peer)),
        Err(e @ PairError::BadCode(_)) => Err(user_error(BAD_CODE)(e)),
        Err(e @ PairError::Unreachable(_)) => Err(user_error(UNREACHABLE)(e)),
        Err(e @ PairError::Failed(_)) => Err(user_error(PAIRING_FAILED)(e)),
    }
}

/// A device just paired. It isn't connected yet: the session is dialed at its saved address.
fn paired_dto(device: &Device, peer: pairing::TrustedPeer) -> TrustedPeerDto {
    TrustedPeerDto {
        words: pairing_words(device, &peer.fingerprint),
        fingerprint: peer.fingerprint,
        display_name: shown_name(peer.display_name),
        paired_at: peer.paired_at,
        is_connected: false,
        endpoint: None,
        status: None,
        wallpaper_color: None,
        wallpaper: None,
    }
}

#[tauri::command]
fn get_permissions(
    state: State<DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Vec<PeerPermissionDto> {
    [
        CapabilityId::FILE_TRANSFER,
        CapabilityId::CLIPBOARD,
        CapabilityId::NOTIFICATIONS,
        CapabilityId::PHOTOS,
        CapabilityId::CALLS,
        CapabilityId::MEDIA,
        CapabilityId::ACTIONS,
        CapabilityId::POINTER,
    ]
    .into_iter()
    .map(|capability| PeerPermissionDto {
        capability_id: capability.raw(),
        allowed: state
            .device
            .stores
            .permissions
            .is_allowed(&peer_fingerprint, capability),
    })
    .collect()
}

#[tauri::command]
fn set_allowed(
    state: State<DesktopRuntimeState>,
    peer_fingerprint: String,
    capability_id: u32,
    allowed: bool,
) -> Result<(), String> {
    state
        .device
        .stores
        .permissions
        .set_allowed(&peer_fingerprint, CapabilityId(capability_id), allowed)
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
        Err(ConnectError::Paused) => {
            Err("Connections are paused. Resume them from Continue's tray icon.".to_string())
        }
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

/// The newest copy for each paired device that wasn't connected when it was made; only the
/// newest matters on a clipboard. Kept until the device connects or the app quits.
#[derive(Default)]
struct CopiesWaiting(Mutex<HashMap<String, Clip>>);

/// Sends what was just copied here to every paired device, now or when it connects.
fn send_copied(app: &AppHandle, clip: Clip) {
    let device = app.state::<DesktopRuntimeState>().device.clone();
    for peer in device.stores.trust.list_peers().unwrap_or_default() {
        if device.sessions.get(&peer.fingerprint).is_some() {
            send_copy(app, peer.fingerprint, clip.clone());
        } else {
            app.state::<CopiesWaiting>()
                .0
                .lock()
                .insert(peer.fingerprint, clip.clone());
        }
    }
}

fn send_copy(app: &AppHandle, peer: String, clip: Clip) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let device = app.state::<DesktopRuntimeState>().device.clone();
        let text = match clip {
            Clip::Text(text) => text,
            Clip::Image(png) => {
                if let Err(error) = device.send_image(&peer, png).await {
                    tracing::warn!("Couldn't send a copied image: {error}");
                }
                return;
            }
        };
        let result = push_text(&device, &peer, text.clone()).await;
        let _ = app.emit(
            "clipboard-synced",
            SyncedTextDto {
                peer_name: shown_name(device.stores.peer_name(&peer)),
                peer_id: peer,
                text,
                failed: result.is_err(),
            },
        );
    });
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

/// Stops the phone forwarding an app's notifications. Unmuting is in the phone's settings.
#[tauri::command]
async fn mute_app(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    package_name: String,
) -> Result<(), String> {
    let mute = notifications::NotificationMute { package_name };
    send_notification(&state, &peer_fingerprint, notifications::Body::Mute(mute)).await
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PhotoDto {
    id: String,
    name: String,
    taken_at: u64,
    thumbnail: String,
}

impl From<protocol::v1::Photo> for PhotoDto {
    fn from(photo: protocol::v1::Photo) -> Self {
        Self {
            thumbnail: jpeg_url(&photo.thumbnail),
            id: photo.id,
            name: photo.name,
            taken_at: photo.taken_at,
        }
    }
}

/// None when the phone hasn't granted photo access.
#[tauri::command]
async fn list_photos(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<Option<Vec<PhotoDto>>, String> {
    let limit = sessions::MAX_PHOTOS;
    let request = photos_message::Body::List(ListPhotos { limit });
    let reply = ask_for_photos(&state, &peer_fingerprint, request).await?;
    let photos = reply.photos.into_iter().map(PhotoDto::from);
    Ok(reply.available.then(|| photos.collect()))
}

/// The photo arrives through the normal file transfer.
#[tauri::command]
async fn get_photo(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    id: String,
) -> Result<(), String> {
    let request = photos_message::Body::Send(SendPhoto { id });
    match ask_for_photos(&state, &peer_fingerprint, request).await? {
        PhotosReply {
            available: true, ..
        } => Ok(()),
        _ => Err("That photo isn't on your phone any more.".to_string()),
    }
}

async fn ask_for_photos(
    state: &DesktopRuntimeState,
    peer: &str,
    request: photos_message::Body,
) -> Result<PhotosReply, String> {
    match state.device.photos(peer, request).await {
        Ok(reply) => Ok(reply),
        Err(SendError::NotConnected) => Err(NOT_CONNECTED.to_string()),
        Err(e) => Err(user_error("Couldn't reach your phone. Try again.")(e)),
    }
}

/// Call audio stays on the phone.
#[tauri::command]
async fn answer_call(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<(), String> {
    act_on_call(&state, &peer_fingerprint, call_action::Kind::Answer).await
}

#[tauri::command]
async fn decline_call(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<(), String> {
    act_on_call(&state, &peer_fingerprint, call_action::Kind::Decline).await
}

#[tauri::command]
async fn silence_call(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<(), String> {
    act_on_call(&state, &peer_fingerprint, call_action::Kind::Silence).await
}

/// Calls a number from the phone; the call itself happens on the phone.
#[tauri::command]
async fn call_number(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    number: String,
) -> Result<(), String> {
    let request = calls_message::Body::Action(CallAction {
        kind: call_action::Kind::Dial.into(),
        number,
    });
    match state.device.calls(&peer_fingerprint, request).await {
        Ok(CallsReply { done: true }) => Ok(()),
        Ok(_) => Err("Your phone couldn't make the call. Turn on Calls in Continue's settings on your phone.".to_string()),
        Err(SendError::NotConnected) => Err(NOT_CONNECTED.to_string()),
        Err(e) => Err(user_error("Couldn't reach your phone. Try again.")(e)),
    }
}

async fn act_on_call(
    state: &DesktopRuntimeState,
    peer: &str,
    kind: call_action::Kind,
) -> Result<(), String> {
    let request = calls_message::Body::Action(CallAction {
        kind: kind.into(),
        number: String::new(),
    });
    match state.device.calls(peer, request).await {
        Ok(CallsReply { done: true }) => Ok(()),
        Ok(_) => Err("Your phone couldn't do that. Try it on the phone.".to_string()),
        Err(SendError::NotConnected) => Err(NOT_CONNECTED.to_string()),
        Err(e) => Err(user_error("Couldn't reach your phone. Try again.")(e)),
    }
}

/// Rings the phone at full volume, even on silent, or stops it.
#[tauri::command]
async fn ring_phone(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    on: bool,
) -> Result<(), String> {
    match state.device.ring(&peer_fingerprint, on).await {
        Ok(true) => Ok(()),
        Ok(false) => {
            Err("Your phone couldn't ring. Check Continue is allowed to ring it.".to_string())
        }
        Err(SendError::NotConnected) => Err(NOT_CONNECTED.to_string()),
        Err(e) => Err(user_error("Couldn't reach your phone. Try again.")(e)),
    }
}

/// Play, pause, skip or change the volume of what's playing on the phone.
#[tauri::command]
async fn media_command(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    command: String,
) -> Result<(), String> {
    use protocol::v1::media_command::Kind;
    let kind =
        Kind::from_str_name(&command).ok_or_else(|| format!("Unknown media command {command}"))?;
    let request = media_message::Body::Command(MediaCommand { kind: kind.into() });
    match state.device.media(&peer_fingerprint, request).await {
        Ok(MediaReply { done: true }) => Ok(()),
        Ok(_) => Err("Nothing is playing on your phone.".to_string()),
        Err(SendError::NotConnected) => Err(NOT_CONNECTED.to_string()),
        Err(e) => Err(user_error("Couldn't reach your phone. Try again.")(e)),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileEntryDto {
    name: String,
    folder: bool,
    size: u64,
    modified: u64,
}

impl From<FileEntry> for FileEntryDto {
    fn from(entry: FileEntry) -> Self {
        Self {
            name: entry.name,
            folder: entry.folder,
            size: entry.size,
            modified: entry.modified,
        }
    }
}

/// None when the phone isn't sharing its files.
#[tauri::command]
async fn list_phone_folder(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    path: String,
) -> Result<Option<Vec<FileEntryDto>>, String> {
    let request = files_message::Body::List(ListFolder { path });
    let reply = ask_for_files(&state, &peer_fingerprint, request).await?;
    let entries = reply.entries.into_iter().map(Into::into);
    Ok(reply.available.then(|| entries.collect()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    kind: &'static str,
    title: String,
    detail: String,
    /// A file's path in the phone's shared folder, or a text's conversation id.
    reference: String,
    at: u64,
}

/// Looks through the phone's files, texts and contacts. The phone does the looking.
#[tauri::command]
async fn search_phone(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    query: String,
) -> Result<Vec<SearchResultDto>, String> {
    use protocol::v1::search_result::Kind as Found;
    let reply = match state.device.search(&peer_fingerprint, query).await {
        Ok(reply) => reply,
        Err(SendError::NotConnected) => return Err(NOT_CONNECTED.to_string()),
        Err(e) => return Err(user_error("Couldn't search your phone. Try again.")(e)),
    };
    if !reply.available {
        return Err(
            "Your phone hasn't let Continue read its files, texts or contacts, or this computer \
             may not search it."
                .to_string(),
        );
    }
    Ok(reply
        .results
        .into_iter()
        .map(|result| SearchResultDto {
            kind: match result.kind() {
                Found::File => "file",
                Found::Text => "text",
                Found::Contact => "contact",
            },
            title: result.title,
            detail: result.detail,
            reference: result.reference,
            at: result.at,
        })
        .collect())
}

/// The file arrives through the normal file transfer.
#[tauri::command]
async fn get_phone_file(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    path: String,
) -> Result<(), String> {
    let request = files_message::Body::Get(GetFile { path });
    match ask_for_files(&state, &peer_fingerprint, request).await? {
        FilesReply {
            available: true, ..
        } => Ok(()),
        _ => Err("That file isn't on your phone any more.".to_string()),
    }
}

async fn ask_for_files(
    state: &DesktopRuntimeState,
    peer: &str,
    request: files_message::Body,
) -> Result<FilesReply, String> {
    match state.device.files(peer, request).await {
        Ok(reply) => Ok(reply),
        Err(SendError::NotConnected) => Err(NOT_CONNECTED.to_string()),
        Err(e) => Err(user_error("Couldn't reach your phone. Try again.")(e)),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConversationDto {
    id: String,
    address: String,
    name: String,
    snippet: String,
    at: u64,
    unread: bool,
}

impl From<Conversation> for ConversationDto {
    fn from(c: Conversation) -> Self {
        Self {
            id: c.id,
            address: c.address,
            name: c.name,
            snippet: c.snippet,
            at: c.at,
            unread: c.unread,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TextMessageDto {
    id: String,
    body: String,
    at: u64,
    outgoing: bool,
}

impl From<TextMessage> for TextMessageDto {
    fn from(t: TextMessage) -> Self {
        Self {
            id: t.id,
            body: t.body,
            at: t.at,
            outgoing: t.outgoing,
        }
    }
}

/// None when the phone hasn't granted SMS access.
#[tauri::command]
async fn list_conversations(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<Option<Vec<ConversationDto>>, String> {
    let limit = sessions::MAX_CONVERSATIONS;
    let request = messages_message::Body::Conversations(ListConversations { limit });
    let reply = ask_for_messages(&state, &peer_fingerprint, request).await?;
    let conversations = reply.conversations.into_iter().map(Into::into);
    Ok(reply.available.then(|| conversations.collect()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactDto {
    name: String,
    number: String,
    favorite: bool,
    /// JPEG data URL.
    photo: Option<String>,
}

/// None when the phone hasn't granted access to its contacts.
#[tauri::command]
async fn list_contacts(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
) -> Result<Option<Vec<ContactDto>>, String> {
    let limit = sessions::MAX_CONTACTS;
    let request = messages_message::Body::Contacts(protocol::v1::ListContacts { limit });
    let reply = ask_for_messages(&state, &peer_fingerprint, request).await?;
    let contacts = reply.contacts.into_iter().map(|contact| ContactDto {
        photo: (!contact.photo.is_empty()).then(|| jpeg_url(&contact.photo)),
        name: contact.name,
        number: contact.number,
        favorite: contact.favorite,
    });
    Ok(reply.available.then(|| contacts.collect()))
}

#[tauri::command]
async fn read_conversation(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    conversation_id: String,
) -> Result<Vec<TextMessageDto>, String> {
    let request = messages_message::Body::Read(ReadConversation {
        conversation_id,
        limit: sessions::MAX_TEXTS,
    });
    let reply = ask_for_messages(&state, &peer_fingerprint, request).await?;
    Ok(reply.messages.into_iter().map(Into::into).collect())
}

#[tauri::command]
async fn send_sms(
    state: State<'_, DesktopRuntimeState>,
    peer_fingerprint: String,
    address: String,
    body: String,
) -> Result<(), String> {
    let request = messages_message::Body::Send(SendText { address, body });
    match ask_for_messages(&state, &peer_fingerprint, request).await? {
        MessagesReply { sent: true, .. } => Ok(()),
        _ => Err("Your phone couldn't send that text.".to_string()),
    }
}

async fn ask_for_messages(
    state: &DesktopRuntimeState,
    peer: &str,
    request: messages_message::Body,
) -> Result<MessagesReply, String> {
    match state.device.messages(peer, request).await {
        Ok(reply) => Ok(reply),
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

fn initialize_desktop_runtime(
    app_handle: &AppHandle,
    db_path: &Path,
    secrets_dir: &Path,
    download_dir: PathBuf,
    clipboard: ClipboardSync,
) -> Result<DesktopRuntimeState, Box<dyn std::error::Error>> {
    let stores = Stores::open(db_path)?;
    let keys = DeviceKeys::load_or_create(&secrets::device_key_store(secrets_dir)?)?;

    let connected = Arc::new(tray::Connected::default());
    let mut handlers = session_handlers(download_dir, &stores, clipboard.clone(), app_handle);
    handlers.on_device_info = Some(device_info_listener(
        app_handle.clone(),
        stores.clone(),
        connected.clone(),
    ));
    let reports = PeerReports::default();
    handlers.on_device_status = Some(device_status_listener(
        app_handle.clone(),
        stores.clone(),
        reports.clone(),
    ));
    handlers.on_device_look = Some(device_look_listener(app_handle.clone(), reports.clone()));
    handlers.computer_actions = Some(Arc::new(actions::DesktopActions(app_handle.clone())));
    handlers.ringer = Some(Arc::new(actions::DesktopRinger(app_handle.clone())));
    handlers.pointer_target = Some(Arc::new(pointer::Touchpad::default()));
    let snippets_app = app_handle.clone();
    handlers.on_snippets_changed = Some(Arc::new(move |_, ()| {
        let _ = snippets_app.emit("snippets-changed", ());
    }));
    let (save_folder, incoming) = (handlers.save_folder.clone(), handlers.incoming.clone());
    let on_state_change = session_state_listener(app_handle.clone(), stores.clone(), connected);
    let device = Device::new(stores, keys, handlers, Some(on_state_change))?;

    Ok(DesktopRuntimeState {
        device_name: computer_name(),
        device,
        active_pairing: Mutex::new(None),
        pending_pair: Mutex::new(None),
        save_folder,
        incoming,
        clipboard,
        reports,
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
        // A second launch focuses the running app and hands over any files (Send to).
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            send_to::queue(app, args);
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

            let device = runtime_state.device.clone();
            remove_old_pastes(app.handle(), &device);
            let paused = paused_marker(app.handle()).is_some_and(|marker| marker.exists());
            tauri::async_runtime::block_on(device.sessions.set_paused(paused));
            look::watch(device.clone());
            // Before listening, as connection handlers read it.
            app.manage(runtime_state);
            app.manage(CopiesWaiting::default());
            // quinn needs a running async runtime to open the endpoint, which `setup` doesn't have.
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

            let handle = app.handle().clone();
            clipboard_watcher.start(move |clip| send_copied(&handle, clip));

            if let Err(error) = tray::create(app.handle(), paused) {
                tracing::warn!("No tray icon, so closing the window will quit: {error}");
            }
            start_at_login_by_default(app.handle(), &app_data);
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("main") {
                without_browser_keys(&window);
            }
            if !std::env::args().any(|arg| arg == BACKGROUND_ARG) {
                tray::show_window(app.handle());
            }
            app.manage(send_to::FilesToSend::default());
            app.manage(video::Watching::default());
            app.manage(pointer::start(app.handle()));
            drop_folder::watch(app.handle().clone());
            app.manage(proximity::start(app.handle()));
            send_to::queue(app.handle(), std::env::args().skip(1));
            send_to::add_to_explorer();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_device_identity,
            get_trusted_peers,
            remove_trusted_peer,
            start_pairing,
            cancel_pairing,
            confirm_pairing,
            pair_from_qr,
            get_permissions,
            set_allowed,
            answer_call,
            decline_call,
            silence_call,
            call_number,
            media_command,
            ring_phone,
            get_history,
            clear_history,
            get_waiting,
            get_snippets,
            pin_snippet,
            unpin_snippet,
            send_later,
            cancel_waiting,
            open_received,
            thumbnail,
            list_photos,
            get_photo,
            send_to::take_files_to_send,
            video::start_mirror,
            video::screen_input,
            pointer::get_phone_side,
            drop_folder::open_drop_folder,
            proximity::get_lock_when_away,
            actions::stop_ringing,
            proximity::set_lock_when_away,
            call_camera::set_call_camera,
            call_camera::call_camera_available,
            pointer::set_phone_side,
            video::stop_mirror,
            video::start_camera,
            video::camera_control,
            video::stop_camera,
            list_phone_folder,
            get_phone_file,
            list_contacts,
            search_phone,
            list_conversations,
            read_conversation,
            send_sms,
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
            mute_app
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
