// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! Documents/Continue Drop: whatever is put there goes to every paired phone, straight away or
//! when it next connects, and moves into Sent so it only goes once.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::DesktopRuntimeState;

const POLL: Duration = Duration::from_secs(2);

pub fn folder(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .document_dir()
        .ok()
        .map(|documents| documents.join("Continue Drop"))
}

#[tauri::command]
pub fn open_drop_folder(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let folder = folder(&app).ok_or("There's no Documents folder to keep it in.")?;
    app.opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

pub fn watch(app: AppHandle) {
    let Some(folder) = folder(&app) else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(folder.join("Sent")) {
        return tracing::warn!("No drop folder: {error}");
    }
    std::thread::spawn(move || {
        let mut sizes = HashMap::new();
        loop {
            std::thread::sleep(POLL);
            let files = files_in(&folder);
            // Only once a file stops growing, so one still being copied in isn't cut short.
            let settled: Vec<PathBuf> = files
                .iter()
                .filter(|(path, size)| sizes.get(path) == Some(size))
                .map(|(path, _)| path.clone())
                .collect();
            sizes = files.into_iter().collect();
            for path in settled {
                sizes.remove(&path);
                if let Some(sent) = move_to_sent(&folder, &path) {
                    tauri::async_runtime::block_on(send(&app, &sent));
                }
            }
        }
    });
}

fn files_in(folder: &Path) -> Vec<(PathBuf, u64)> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            metadata.is_file().then(|| (entry.path(), metadata.len()))
        })
        .collect()
}

/// Into Sent, numbered if a file of that name went before.
fn move_to_sent(folder: &Path, path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem.to_string(), format!(".{extension}")),
        _ => (name.clone(), String::new()),
    };
    let sent = (1..)
        .map(|n| match n {
            1 => folder.join("Sent").join(&name),
            n => folder.join("Sent").join(format!("{stem} ({n}){extension}")),
        })
        .find(|candidate| !candidate.exists())?;
    std::fs::rename(path, &sent)
        .map_err(|error| tracing::warn!("Couldn't move {name} to Sent: {error}"))
        .ok()?;
    Some(sent)
}

async fn send(app: &AppHandle, path: &Path) {
    let device = app.state::<DesktopRuntimeState>().device.clone();
    let location = path.to_string_lossy().into_owned();
    for peer in device.stores.trust.list_peers().unwrap_or_default() {
        let peer = peer.fingerprint;
        let result = if device.sessions.get(&peer).is_some() {
            device
                .send_file(&peer, path, Some(location.clone()), None::<fn(u64, u64)>)
                .await
                .map(drop)
                .map_err(|error| error.to_string())
        } else {
            device
                .send_later(&peer, history::Kind::File, &location)
                .map(drop)
                .map_err(|error| error.to_string())
        };
        if let Err(error) = result {
            tracing::warn!("Couldn't send {location} from the drop folder: {error}");
        }
    }
}
