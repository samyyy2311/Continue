// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! The phone's camera as a webcam other apps can pick, while the Webcam panel is open with it
//! switched on. Windows 11 only.

use std::sync::mpsc::Sender;
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::State;

use crate::video::Watching;

/// Installs made with the MSI don't carry the camera DLL.
#[cfg(windows)]
const NO_CAMERA_DLL: &str = "This copy of Continue came without its camera. \
    Install it with the setup .exe to use your phone in video calls.";

/// H.264 from the phone and its quarter turns, on their way to the thread that shows them.
type Frame = (Vec<u8>, u32);

#[derive(Clone, Default)]
pub struct CallCamera(Arc<Mutex<Option<Sender<Frame>>>>);

impl CallCamera {
    pub fn push(&self, h264: &[u8], quarter_turns: u32) {
        if let Some(frames) = self.0.lock().as_ref() {
            let _ = frames.send((h264.to_vec(), quarter_turns));
        }
    }

    /// Takes the camera off the system.
    pub fn stop(&self) {
        self.0.lock().take();
    }

    #[cfg(windows)]
    fn start(&self) -> Result<(), String> {
        use continue_camera::{StartError, Webcam};
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

        // Installed, it's beside the exe; in a development build, with the other dependencies.
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let dll = ["continue_camera.dll", "deps/continue_camera.dll"]
            .into_iter()
            .map(|name| exe.with_file_name(name))
            .find(|path| path.exists())
            .ok_or(NO_CAMERA_DLL)?;
        let (frames, queued) = std::sync::mpsc::channel::<Frame>();
        let (started, result) = std::sync::mpsc::channel();
        // The camera and decoder are COM objects, so one thread makes and uses them.
        std::thread::spawn(move || {
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            let mut webcam = match Webcam::start(&dll) {
                Ok(webcam) => {
                    let _ = started.send(Ok(()));
                    webcam
                }
                Err(error) => {
                    let message = match error {
                        StartError::Unsupported => "Using your phone in video calls needs Windows 11.",
                        StartError::Declined => {
                            "Continue needs your permission to add its camera. Try again and choose Yes."
                        }
                        StartError::Failed(error) => {
                            tracing::warn!("Couldn't add the camera: {error}");
                            "Couldn't add Continue's camera."
                        }
                    };
                    let _ = started.send(Err(message.to_string()));
                    return;
                }
            };
            for (h264, quarter_turns) in queued {
                if let Err(error) = webcam.push(&h264, quarter_turns) {
                    tracing::debug!("Skipped a frame for the camera: {error}");
                }
            }
        });
        result.recv().map_err(|e| e.to_string())??;
        *self.0.lock() = Some(frames);
        Ok(())
    }

    #[cfg(not(windows))]
    fn start(&self) -> Result<(), String> {
        Err("Using your phone in video calls is only on Windows for now.".to_string())
    }
}

/// Turning it on can ask Windows for permission the first time, so this waits for that.
#[tauri::command]
pub async fn set_call_camera(watching: State<'_, Watching>, on: bool) -> Result<(), String> {
    let camera = watching.call_camera.clone();
    if !on {
        camera.stop();
        return Ok(());
    }
    tauri::async_runtime::spawn_blocking(move || camera.start())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn call_camera_available() -> bool {
    cfg!(windows)
}
