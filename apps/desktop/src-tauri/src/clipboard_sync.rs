// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! Keeps the clipboard in step with connected phones: text copied here goes to them, and
//! text they send lands on the clipboard here, whether or not the window is in front.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use clipboard_rs::{Clipboard, ClipboardContext};

/// How often the clipboard is checked for something newly copied.
const POLL: Duration = Duration::from_millis(700);

/// Formats password managers add to say "don't sync or record this".
const CONCEALED: [&str; 3] = [
    "org.nspasteboard.ConcealedType",               // macOS
    "ExcludeClipboardContentFromMonitorProcessing", // Windows
    "x-kde-passwordManagerHint",                    // Linux
];

/// The handle the rest of the app uses. Cheap to clone.
#[derive(Clone)]
pub struct ClipboardSync {
    writes: Sender<String>,
    enabled: Arc<AtomicBool>,
}

/// The half that runs the clipboard thread, started once sending is possible.
pub struct ClipboardWatcher {
    writes: Receiver<String>,
    enabled: Arc<AtomicBool>,
}

/// Creates a clipboard synchronization handle and its watcher.
///
/// Clipboard synchronization is disabled initially. The returned handle and
/// watcher share the same enabled state and communicate through a channel.
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
    /// Queues text received from a phone for writing to this computer's clipboard.
    ///
    /// The text is not sent back to the phone. Queueing failures are ignored.
    pub fn write(&self, text: String) {
        let _ = self.writes.send(text);
    }

    /// Enables or disables sending locally copied text.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

impl ClipboardWatcher {
    /// Starts watching the system clipboard and passes eligible copied text to `on_copy`.
    
    ///
    
    /// The callback is invoked for changed, nonblank clipboard text when syncing is enabled
    
    /// and the clipboard content is not marked as concealed.
    pub fn start(self, on_copy: impl Fn(String) + Send + 'static) {
        let started = std::thread::Builder::new()
            .name("clipboard".into())
            .spawn(move || self.run(on_copy));
        if let Err(error) = started {
            tracing::warn!("Couldn't start watching the clipboard: {error}");
        }
    }

    /// Watches for new clipboard text and forwards eligible copies to `on_copy`.
    ///
    /// Forwards text only when syncing is enabled, the text is nonblank, and the
    /// clipboard does not contain a concealed format. Ignores the clipboard text
    /// present at startup and text written through the incoming channel.դրբեջ
    fn run(self, on_copy: impl Fn(String)) {
        let clipboard = match ClipboardContext::new() {
            Ok(clipboard) => clipboard,
            Err(error) => {
                tracing::warn!("Clipboard sync is unavailable: {error}");
                return;
            }
        };
        // What's on the clipboard at start was copied before; only new copies are sent.
        let mut last = clipboard.get_text().unwrap_or_default();
        loop {
            match self.writes.recv_timeout(POLL) {
                Ok(text) => match clipboard.set_text(text.clone()) {
                    Ok(()) => last = text,
                    Err(error) => tracing::warn!("Couldn't copy received text: {error}"),
                },
                Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {
                    let Ok(text) = clipboard.get_text() else {
                        continue;
                    };
                    if text == last {
                        continue;
                    }
                    last = text.clone();
                    let wanted = self.enabled.load(Ordering::Relaxed) && !text.trim().is_empty();
                    if wanted && !is_concealed(&clipboard) {
                        on_copy(text);
                    }
                }
            }
        }
    }
}

/// Determines whether the clipboard contains a format marked as concealed.
///
/// # Returns
///
/// `true` if any available clipboard format is concealed, `false` otherwise. Returns `false` if the available formats cannot be queried.
fn is_concealed(clipboard: &ClipboardContext) -> bool {
    clipboard
        .available_formats()
        .is_ok_and(|formats| formats.iter().any(|f| CONCEALED.contains(&f.as_str())))
}
