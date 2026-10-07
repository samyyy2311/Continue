// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Files passed to Continue from outside the window, mainly Explorer's Send to menu.

use parking_lot::Mutex;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
pub struct FilesToSend(Mutex<Vec<String>>);

pub fn queue(app: &AppHandle, args: impl IntoIterator<Item = String>) {
    let files: Vec<String> = args
        .into_iter()
        .filter(|arg| Path::new(arg).is_file())
        .collect();
    if files.is_empty() {
        return;
    }
    app.state::<FilesToSend>().0.lock().extend(files);
    let _ = app.emit("files-to-send", ());
}

#[tauri::command]
pub fn take_files_to_send(files: State<FilesToSend>) -> Vec<String> {
    std::mem::take(&mut *files.0.lock())
}

/// Puts Continue in Explorer's "Send to" menu, pointing at this copy of the app. Windows only for now.
#[cfg(windows)]
pub fn add_to_explorer() {
    use windows::core::{Interface, HSTRING};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    let (Some(app_data), Ok(exe)) = (std::env::var_os("APPDATA"), std::env::current_exe()) else {
        return;
    };
    let shortcut = Path::new(&app_data).join(r"Microsoft\Windows\SendTo\Continue.lnk");
    // Its own thread, so COM is set up apart from the window's.
    std::thread::spawn(move || unsafe {
        let made = (|| -> windows::core::Result<()> {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            link.SetPath(&HSTRING::from(exe.as_os_str()))?;
            link.SetDescription(&HSTRING::from("Send to your phone"))?;
            link.cast::<IPersistFile>()?
                .Save(&HSTRING::from(shortcut.as_os_str()), true)
        })();
        if let Err(error) = made {
            tracing::warn!("Couldn't add Continue to Send to: {error}");
        }
    });
}

#[cfg(not(windows))]
pub fn add_to_explorer() {}
