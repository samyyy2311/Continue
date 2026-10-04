// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! Keeps Continue in the system tray, so closing the window doesn't stop it receiving, and
//! says there who is connected.

use std::collections::BTreeMap;

use parking_lot::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const TRAY_ID: &str = "main";

/// Creates the system-tray menu and icon for the application.
///
/// The Open action and a left-button release on the tray icon show the main window; the Quit action exits the application. Uses the default window icon when available.
///
/// # Errors
///
/// Returns an error if creating the menu items, menu, or tray icon fails.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Continue", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Continue · No phone connected")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Determines whether the application has a tray icon with the configured ID.
///
/// # Returns
///
/// `true` if the tray icon exists, `false` otherwise.
pub fn exists(app: &AppHandle) -> bool {
    app.tray_by_id(TRAY_ID).is_some()
}

/// Shows, unminimizes, and focuses the main window when it exists.】【。
pub fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Who's connected, by fingerprint, for the tray's tooltip.
#[derive(Default)]
pub struct Connected(Mutex<BTreeMap<String, String>>);

impl Connected {
    /// Updates a peer's connection state and the tray tooltip.
    ///
    /// When `connected` is `true`, stores `name` for `peer`; otherwise, removes `peer`.
    /// The tooltip reflects the resulting set of connected peers. Tooltip update errors
    /// are ignored.
    ///
    /// # Parameters
    /// - `peer`: The peer's fingerprint, used as its map key.
    /// - `name`: The name to display when the peer is connected.
    /// - `connected`: Whether to add or remove the peer.
    pub fn update(&self, app: &AppHandle, peer: &str, name: String, connected: bool) {
        let mut peers = self.0.lock();
        if connected {
            peers.insert(peer.to_string(), name);
        } else {
            peers.remove(peer);
        }
        let status = match peers.len() {
            0 => "Continue · No phone connected".to_string(),
            1 => format!(
                "Continue · Connected to {}",
                peers.values().next().map_or("", String::as_str)
            ),
            n => format!("Continue · Connected to {n} devices"),
        };
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            let _ = tray.set_tooltip(Some(status));
        }
    }
}
