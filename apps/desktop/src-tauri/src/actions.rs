// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! What a paired phone can have this computer do: lock it, type into it, open a link on it.

use protocol::v1::computer_action::Body;
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

pub struct DesktopActions(pub AppHandle);

impl sessions::ComputerActions for DesktopActions {
    fn act(&self, action: Body) -> bool {
        match action {
            Body::Lock(_) => lock(),
            Body::Sleep(_) => {
                // Sleeping returns only on waking, so the phone hears back first.
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    sleep();
                });
                true
            }
            Body::TypeText(text) => type_text(&text),
            Body::OpenLink(link) => open_link(&self.0, &link),
        }
    }
}

/// Rings this computer when the phone looks for it: the window comes up with a banner, and on
/// Windows its alarm sound plays until stopped.
pub struct DesktopRinger(pub AppHandle);

impl sessions::Ringer for DesktopRinger {
    fn ring(&self, on: bool) -> bool {
        if on {
            crate::tray::show_window(&self.0);
        }
        alarm(on);
        let _ = self.0.emit("ring", on);
        true
    }
}

#[tauri::command]
pub fn stop_ringing(app: AppHandle) {
    alarm(false);
    let _ = app.emit("ring", false);
}

#[cfg(windows)]
fn alarm(on: bool) {
    use windows::core::HSTRING;
    use windows::Win32::Media::Audio::{
        PlaySoundW, SND_ASYNC, SND_FILENAME, SND_LOOP, SND_NODEFAULT,
    };
    unsafe {
        if on {
            let windows = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
            let sound = HSTRING::from(format!(r"{windows}\Media\Alarm01.wav"));
            let flags = SND_FILENAME | SND_ASYNC | SND_LOOP | SND_NODEFAULT;
            let _ = PlaySoundW(&sound, None, flags);
        } else {
            let _ = PlaySoundW(None, None, SND_ASYNC);
        }
    }
}

#[cfg(not(windows))]
fn alarm(_: bool) {}

/// Only web links, so a phone can't start programs or open files here.
fn open_link(app: &AppHandle, link: &str) -> bool {
    let web = link.starts_with("https://") || link.starts_with("http://");
    web && app.opener().open_url(link, None::<&str>).is_ok()
}

#[cfg(windows)]
fn sleep() {
    // Sleep rather than hibernate, and let programs that keep it awake say no.
    unsafe { windows::Win32::System::Power::SetSuspendState(false, false, false) };
}

#[cfg(target_os = "macos")]
fn sleep() {
    let _ = std::process::Command::new("pmset").arg("sleepnow").status();
}

#[cfg(target_os = "linux")]
fn sleep() {
    let _ = std::process::Command::new("systemctl")
        .arg("suspend")
        .status();
}

#[cfg(windows)]
fn lock() -> bool {
    unsafe { windows::Win32::System::Shutdown::LockWorkStation() }.is_ok()
}

#[cfg(target_os = "macos")]
fn lock() -> bool {
    // Sleeping the display locks it when the Mac asks for a password on wake, as it does by default.
    std::process::Command::new("pmset")
        .arg("displaysleepnow")
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(target_os = "linux")]
fn lock() -> bool {
    std::process::Command::new("loginctl")
        .arg("lock-session")
        .status()
        .is_ok_and(|status| status.success())
}

/// Types as key presses, so it works in any app, whatever the keyboard layout.
#[cfg(windows)]
pub(crate) fn type_text(text: &str) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
        KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_RETURN,
    };

    let key = |vk: VIRTUAL_KEY, unit: u16, flags: KEYBD_EVENT_FLAGS| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: unit,
                dwFlags: flags,
                ..Default::default()
            },
        },
    };
    let mut inputs = Vec::new();
    for unit in text.replace("\r\n", "\n").encode_utf16() {
        // Enter as a real key, since apps don't take a typed line break as one.
        let (vk, unit, flags) = if unit == u16::from(b'\n') {
            (VK_RETURN, 0, KEYBD_EVENT_FLAGS(0))
        } else {
            (VIRTUAL_KEY(0), unit, KEYEVENTF_UNICODE)
        };
        inputs.push(key(vk, unit, flags));
        inputs.push(key(vk, unit, flags | KEYEVENTF_KEYUP));
    }
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    sent as usize == inputs.len()
}

#[cfg(not(windows))]
pub(crate) fn type_text(_: &str) -> bool {
    false
}
