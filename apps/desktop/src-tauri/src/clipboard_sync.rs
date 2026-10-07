// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! Keeps the clipboard in step with connected phones: text and images copied here go to them,
//! and what they send lands on the clipboard here, whether or not the window is in front.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext, ContentFormat, RustImageData};

/// How often the clipboard is checked for something newly copied.
const POLL: Duration = Duration::from_millis(700);

/// Formats password managers add to say "don't sync or record this".
const CONCEALED: [&str; 3] = [
    "org.nspasteboard.ConcealedType",               // macOS
    "ExcludeClipboardContentFromMonitorProcessing", // Windows
    "x-kde-passwordManagerHint",                    // Linux
];

#[derive(Clone, PartialEq)]
pub enum Clip {
    Text(String),
    /// PNG.
    Image(Vec<u8>),
}

/// The handle the rest of the app uses. Cheap to clone.
#[derive(Clone)]
pub struct ClipboardSync {
    writes: Sender<Clip>,
    enabled: Arc<AtomicBool>,
}

/// The half that runs the clipboard thread, started once sending is possible.
pub struct ClipboardWatcher {
    writes: Receiver<Clip>,
    enabled: Arc<AtomicBool>,
}

pub fn new() -> (ClipboardSync, ClipboardWatcher) {
    let (writes, requests) = mpsc::channel();
    // Off until the window says what the user chose.
    let enabled = Arc::new(AtomicBool::new(false));
    (
        ClipboardSync {
            writes,
            enabled: enabled.clone(),
        },
        ClipboardWatcher {
            writes: requests,
            enabled,
        },
    )
}

impl ClipboardSync {
    /// Puts what a phone sent on this computer's clipboard. It isn't sent back.
    pub fn write(&self, clip: Clip) {
        let _ = self.writes.send(clip);
    }

    /// Turns sending what's copied here on or off.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

impl ClipboardWatcher {
    /// Starts watching. `on_copy` gets what's copied here while sending is on.
    pub fn start(self, on_copy: impl Fn(Clip) + Send + 'static) {
        let started = std::thread::Builder::new()
            .name("clipboard".into())
            .spawn(move || self.run(on_copy));
        if let Err(error) = started {
            tracing::warn!("Couldn't start watching the clipboard: {error}");
        }
    }

    /// One thread owns the clipboard, because on Linux what this app copies only stays on
    /// the clipboard while the context that copied it is alive.
    fn run(self, on_copy: impl Fn(Clip)) {
        let clipboard = match ClipboardContext::new() {
            Ok(clipboard) => clipboard,
            Err(error) => {
                tracing::warn!("Clipboard sync is unavailable: {error}");
                return;
            }
        };
        // What's on the clipboard at start was copied before; only new copies are sent.
        let mut last = current(&clipboard);
        let mut seen = change_count();
        loop {
            match self.writes.recv_timeout(POLL) {
                Ok(clip) => {
                    match write(&clipboard, &clip) {
                        Ok(()) => last = Some(clip),
                        Err(error) => tracing::warn!("Couldn't copy what a phone sent: {error}"),
                    }
                    seen = change_count();
                }
                Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {
                    let count = change_count();
                    if count.is_some() && count == seen {
                        continue;
                    }
                    seen = count;
                    let Some(clip) = current(&clipboard) else {
                        continue;
                    };
                    if last.as_ref() == Some(&clip) {
                        continue;
                    }
                    last = Some(clip.clone());
                    if self.enabled.load(Ordering::Relaxed) && !is_concealed(&clipboard) {
                        on_copy(clip);
                    }
                }
            }
        }
    }
}

fn current(clipboard: &ClipboardContext) -> Option<Clip> {
    if let Ok(text) = clipboard.get_text() {
        if !text.trim().is_empty() {
            return Some(Clip::Text(text));
        }
    }
    // Reading an image means converting it to PNG, too costly on every poll without a change count.
    if change_count().is_none() || !clipboard.has(ContentFormat::Image) {
        return None;
    }
    let png = clipboard.get_image().ok()?.to_png().ok()?;
    Some(Clip::Image(png.get_bytes().to_vec()))
}

fn write(clipboard: &ClipboardContext, clip: &Clip) -> clipboard_rs::Result<()> {
    match clip {
        Clip::Text(text) => clipboard.set_text(text.clone()),
        Clip::Image(png) => clipboard.set_image(RustImageData::from_bytes(png)?),
    }
}

/// Bumped by Windows on every clipboard change. Other systems aren't wired up, so they only sync text.
#[cfg(windows)]
fn change_count() -> Option<u32> {
    Some(unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() })
}

#[cfg(not(windows))]
fn change_count() -> Option<u32> {
    None
}

fn is_concealed(clipboard: &ClipboardContext) -> bool {
    clipboard
        .available_formats()
        .is_ok_and(|formats| formats.iter().any(|f| CONCEALED.contains(&f.as_str())))
}
