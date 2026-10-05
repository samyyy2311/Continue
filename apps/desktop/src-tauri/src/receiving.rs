// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! Files on their way in, and the folder they're saved to.

use std::path::{Path, PathBuf};

use device::Stores;
use serde::Serialize;
use sessions::{IncomingEvent, IncomingFile, IncomingListener};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{user_error, DesktopRuntimeState};

/// Remembers the chosen folder between runs.
const SAVE_FOLDER_FILE: &str = "save-folder";

/// A file coming in, as the window shows it.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingDto {
    transfer_id: String,
    peer_id: String,
    peer_name: String,
    file_name: String,
    received: u64,
    total: u64,
}

fn to_dto(file: IncomingFile, stores: &Stores) -> IncomingDto {
    IncomingDto {
        peer_name: stores.peer_name(&file.peer),
        transfer_id: file.transfer_id,
        peer_id: file.peer,
        file_name: file.file_name,
        received: file.received,
        total: file.total,
    }
}

/// Tells the window as files start, move along and stop.
pub fn listener(app: AppHandle, stores: Stores) -> IncomingListener {
    std::sync::Arc::new(move |event| {
        let _ = match event {
            IncomingEvent::Progress(file) => app.emit("incoming-progress", to_dto(file, &stores)),
            IncomingEvent::Ended { transfer_id, .. } => app.emit("incoming-ended", transfer_id),
        };
    })
}

#[tauri::command]
pub fn list_incoming(state: State<DesktopRuntimeState>) -> Vec<IncomingDto> {
    let files = state.incoming.list();
    files
        .into_iter()
        .map(|file| to_dto(file, &state.device.stores))
        .collect()
}

#[tauri::command]
pub fn cancel_incoming(state: State<DesktopRuntimeState>, transfer_id: String) {
    // Already finished is fine: there's nothing left to stop.
    state.incoming.cancel(&transfer_id);
}

/// Downloads, unless the user chose somewhere else.
pub fn starting_folder(app: &AppHandle, app_data: &Path) -> PathBuf {
    let chosen = std::fs::read_to_string(app_data.join(SAVE_FOLDER_FILE))
        .ok()
        .map(|text| PathBuf::from(text.trim()))
        .filter(|folder| folder.is_dir());
    chosen.unwrap_or_else(|| default_folder(app))
}

fn default_folder(app: &AppHandle) -> PathBuf {
    let path = app.path();
    path.download_dir()
        .or_else(|_| path.app_data_dir().map(|dir| dir.join("downloads")))
        .unwrap_or_else(|_| PathBuf::from("downloads"))
}

#[tauri::command]
pub fn get_save_folder(state: State<DesktopRuntimeState>) -> String {
    state.save_folder.get().to_string_lossy().into_owned()
}

/// Saves received files to `folder` from now on, or to Downloads when it's `None`. Returns
/// the folder now in use.
#[tauri::command]
pub fn set_save_folder(
    app: AppHandle,
    state: State<DesktopRuntimeState>,
    folder: Option<String>,
) -> Result<String, String> {
    const FAILED: &str = "Couldn't use that folder. Pick one you can save to.";
    let folder = folder.map_or_else(|| default_folder(&app), PathBuf::from);
    std::fs::create_dir_all(&folder).map_err(user_error(FAILED))?;
    // Proves the folder takes new files before anything is sent there.
    let probe = folder.join(".continue-write-check");
    std::fs::write(&probe, b"")
        .and_then(|()| std::fs::remove_file(&probe))
        .map_err(user_error(FAILED))?;
    let app_data = app.path().app_data_dir().map_err(user_error(FAILED))?;
    std::fs::write(
        app_data.join(SAVE_FOLDER_FILE),
        folder.to_string_lossy().as_bytes(),
    )
    .map_err(user_error(FAILED))?;
    state.save_folder.set(&folder);
    Ok(folder.to_string_lossy().into_owned())
}
