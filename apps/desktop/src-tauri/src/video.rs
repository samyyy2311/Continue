// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! The phone's screen and camera in the window.

use protocol::v1::{
    camera_control, screen_input, touch, CameraControl, CameraRequest, ScreenButton, ScreenInput,
    ScreenKey, ScreenRequest, Swipe, Touch, VideoStart,
};
use serde::{Deserialize, Serialize};
use sessions::{VideoControl, Watched};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::call_camera::CallCamera;
use crate::{DesktopRuntimeState, NOT_CONNECTED};

/// As sharp as most phone screens; the phone never sends more than its own size.
const SCREEN_MAX_SIZE: u32 = 2400;
/// Full HD, what video calls use at most.
const CAMERA_MAX_SIZE: u32 = 1920;

#[derive(Default)]
pub struct Watching {
    screen: Mutex<Option<VideoControl<ScreenInput>>>,
    camera: Mutex<Option<VideoControl<CameraControl>>>,
    pub call_camera: CallCamera,
}

#[derive(Serialize)]
pub struct VideoSizeDto {
    width: u32,
    height: u32,
}

/// Sends each frame to `on_frame` as a byte saying what it is (0 a frame, 1 a key frame, 2 sound),
/// a byte with its quarter turns, then the H.264 or PCM data, and tells the window `ended` once
/// the phone stops. Frames also go to `call_camera` while it's on.
fn forward<C: Send + 'static>(
    app: AppHandle,
    watched: Option<Watched<C>>,
    slot: &mut Option<VideoControl<C>>,
    on_frame: Channel<InvokeResponseBody>,
    ended: &'static str,
    call_camera: Option<CallCamera>,
) -> Result<VideoSizeDto, String> {
    let Some((VideoStart { width, height, .. }, mut frames, control)) = watched else {
        return Err("Your phone didn't share it.".to_string());
    };
    *slot = Some(control);
    tauri::async_runtime::spawn(async move {
        while let Ok(frame) = frames.next().await {
            if !frame.audio.is_empty() {
                let mut message = Vec::with_capacity(frame.audio.len() + 2);
                message.extend_from_slice(&[2, 0]);
                message.extend_from_slice(&frame.audio);
                if on_frame.send(InvokeResponseBody::Raw(message)).is_err() {
                    break;
                }
                continue;
            }
            let quarter_turns = (frame.rotation / 90 % 4) as u8;
            if let Some(camera) = &call_camera {
                camera.push(&frame.data, u32::from(quarter_turns));
            }
            let mut message = Vec::with_capacity(frame.data.len() + 2);
            message.extend_from_slice(&[u8::from(frame.key), quarter_turns]);
            message.extend_from_slice(&frame.data);
            if on_frame.send(InvokeResponseBody::Raw(message)).is_err() {
                break;
            }
        }
        if let Some(camera) = call_camera {
            camera.stop();
        }
        let _ = app.emit(ended, ());
    });
    Ok(VideoSizeDto { width, height })
}

fn reach(e: impl std::fmt::Display) -> String {
    format!("Couldn't reach your phone. Try again. ({e})")
}

/// Starts showing the phone's screen; the window hears `mirror-ended` when it stops.
#[tauri::command]
pub async fn start_mirror(
    app: AppHandle,
    state: State<'_, DesktopRuntimeState>,
    watching: State<'_, Watching>,
    peer_fingerprint: String,
    on_frame: Channel<InvokeResponseBody>,
) -> Result<VideoSizeDto, String> {
    let mux = state
        .device
        .sessions
        .get(&peer_fingerprint)
        .ok_or(NOT_CONNECTED)?;
    let request = ScreenRequest {
        max_size: SCREEN_MAX_SIZE,
    };
    let watched = mux.watch_screen(request).await.map_err(reach)?;
    forward(
        app,
        watched,
        &mut *watching.screen.lock().await,
        on_frame,
        "mirror-ended",
        None,
    )
}

/// Starts the phone's camera as a webcam; the window hears `camera-ended` when it stops.
#[tauri::command]
pub async fn start_camera(
    app: AppHandle,
    state: State<'_, DesktopRuntimeState>,
    watching: State<'_, Watching>,
    peer_fingerprint: String,
    front: bool,
    on_frame: Channel<InvokeResponseBody>,
) -> Result<VideoSizeDto, String> {
    let mux = state
        .device
        .sessions
        .get(&peer_fingerprint)
        .ok_or(NOT_CONNECTED)?;
    let request = CameraRequest {
        max_size: CAMERA_MAX_SIZE,
        front,
    };
    let watched = mux.watch_camera(request).await.map_err(reach)?;
    forward(
        app,
        watched,
        &mut *watching.camera.lock().await,
        on_frame,
        "camera-ended",
        Some(watching.call_camera.clone()),
    )
}

/// Something to do on the phone's screen. Positions are fractions of the screen.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ScreenInputDto {
    Down {
        x: f32,
        y: f32,
    },
    Move {
        x: f32,
        y: f32,
    },
    Up {
        x: f32,
        y: f32,
    },
    Swipe {
        from_x: f32,
        from_y: f32,
        to_x: f32,
        to_y: f32,
        duration_ms: u32,
    },
    Back,
    Home,
    Recents,
    Text {
        text: String,
    },
    Backspace,
    Enter,
}

#[tauri::command]
pub async fn screen_input(
    watching: State<'_, Watching>,
    input: ScreenInputDto,
) -> Result<(), String> {
    let touching = |action: touch::Action, x, y| {
        screen_input::Body::Touch(Touch {
            action: action.into(),
            x,
            y,
        })
    };
    let input = match input {
        ScreenInputDto::Down { x, y } => touching(touch::Action::Down, x, y),
        ScreenInputDto::Move { x, y } => touching(touch::Action::Move, x, y),
        ScreenInputDto::Up { x, y } => touching(touch::Action::Up, x, y),
        ScreenInputDto::Swipe {
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
        } => screen_input::Body::Swipe(Swipe {
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
        }),
        ScreenInputDto::Back => screen_input::Body::Button(ScreenButton::Back.into()),
        ScreenInputDto::Home => screen_input::Body::Button(ScreenButton::Home.into()),
        ScreenInputDto::Recents => screen_input::Body::Button(ScreenButton::Recents.into()),
        ScreenInputDto::Text { text } => screen_input::Body::Text(text),
        ScreenInputDto::Backspace => screen_input::Body::Key(ScreenKey::Backspace.into()),
        ScreenInputDto::Enter => screen_input::Body::Key(ScreenKey::Enter.into()),
    };
    send(&watching.screen, ScreenInput { body: Some(input) }).await
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CameraControlDto {
    Front { front: bool },
    Framing { on: bool },
}

#[tauri::command]
pub async fn camera_control(
    watching: State<'_, Watching>,
    control: CameraControlDto,
) -> Result<(), String> {
    let body = match control {
        CameraControlDto::Front { front } => camera_control::Body::Front(front),
        CameraControlDto::Framing { on } => camera_control::Body::Framing(on),
    };
    send(&watching.camera, CameraControl { body: Some(body) }).await
}

async fn send<C: prost::Message>(
    slot: &Mutex<Option<VideoControl<C>>>,
    control: C,
) -> Result<(), String> {
    match slot.lock().await.as_mut() {
        Some(watched) => watched.send(&control).await.map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

/// Dropping the control stops the phone's side too.
#[tauri::command]
pub async fn stop_mirror(watching: State<'_, Watching>) -> Result<(), String> {
    watching.screen.lock().await.take();
    Ok(())
}

#[tauri::command]
pub async fn stop_camera(watching: State<'_, Watching>) -> Result<(), String> {
    watching.call_camera.stop();
    watching.camera.lock().await.take();
    Ok(())
}
