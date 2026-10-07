// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0-only

//! Takes the mouse and keyboard over to the phone when the pointer is pushed past the screen
//! edge the phone sits by, and back when it comes off the phone's edge or Esc is pressed.
//! Windows only for now.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use tauri::{AppHandle, Manager, State};

#[derive(Clone, Copy, PartialEq)]
pub enum Side {
    Left,
    Right,
}

/// Which side of the screens the phone sits on; None while this is off.
#[derive(Clone, Default)]
pub struct PhoneSide(Arc<Mutex<Option<Side>>>);

fn saved_side(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("phone-side"))
}

/// Reads the saved side, then watches for the pointer reaching it.
pub fn start(app: &AppHandle) -> PhoneSide {
    let side = saved_side(app)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|side| match side.trim() {
            "left" => Some(Side::Left),
            "right" => Some(Side::Right),
            _ => None,
        });
    let phone_side = PhoneSide(Arc::new(Mutex::new(side)));
    #[cfg(windows)]
    windows_input::watch(app.clone(), phone_side.clone());
    phone_side
}

/// The phone used as this computer's touchpad: its input moves the pointer and types here.
/// Windows only for now.
#[derive(Default)]
pub struct Touchpad {
    /// What's left of each move after whole pixels, carried into the next.
    remainder: Mutex<(f32, f32)>,
}

impl sessions::PointerTarget for Touchpad {
    fn start(&self, _: protocol::v1::PointerStart, _: sessions::PointerLeave) -> bool {
        *self.remainder.lock() = (0.0, 0.0);
        cfg!(windows)
    }

    fn input(&self, input: protocol::v1::PointerInput) {
        #[cfg(windows)]
        windows_input::inject(input, &mut self.remainder.lock());
        #[cfg(not(windows))]
        let _ = input;
    }

    /// Lets go of anything still held, in case the phone went mid-drag.
    fn stop(&self) {
        #[cfg(windows)]
        windows_input::release_buttons();
    }
}

/// "off", "left" or "right"; None where this isn't available.
#[tauri::command]
pub fn get_phone_side(side: State<PhoneSide>) -> Option<&'static str> {
    if !cfg!(windows) {
        return None;
    }
    Some(match *side.0.lock() {
        None => "off",
        Some(Side::Left) => "left",
        Some(Side::Right) => "right",
    })
}

#[tauri::command]
pub fn set_phone_side(app: AppHandle, side: State<PhoneSide>, choice: String) {
    let chosen = match choice.as_str() {
        "left" => Some(Side::Left),
        "right" => Some(Side::Right),
        _ => None,
    };
    *side.0.lock() = chosen;
    if let Some(path) = saved_side(&app) {
        let saved = match chosen {
            Some(_) => std::fs::write(path, &choice),
            None => std::fs::remove_file(path),
        };
        if let Err(error) = saved {
            tracing::warn!("Couldn't save where the phone sits: {error}");
        }
    }
}

#[cfg(windows)]
mod windows_input {
    use std::time::Duration;

    use parking_lot::Mutex;
    use protocol::v1::{
        pointer_input::Body, screen_input, PointerInput, PointerMove, PointerStart, ScreenButton,
        ScreenInput, ScreenKey,
    };
    use tauri::{AppHandle, Emitter, Manager};
    use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
    use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyState, ToUnicode, VIRTUAL_KEY, VK_BACK, VK_CAPITAL, VK_CONTROL,
        VK_ESCAPE, VK_LBUTTON, VK_MENU, VK_RETURN, VK_SHIFT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetCursorPos, GetMessageW, GetSystemMetrics, PostThreadMessageW,
        SetCursorPos, SetWindowsHookExW, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT,
        SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
        WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONUP,
        WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT, WM_RBUTTONUP, WM_SYSKEYDOWN,
    };

    use super::{PhoneSide, Side};
    use crate::DesktopRuntimeState;

    const POLL: Duration = Duration::from_millis(15);
    /// How far inside the edge the pointer is held while it's on the phone, so moving further
    /// that way isn't stopped by the edge.
    const HOLD_INSET: i32 = 100;

    enum Event {
        Input(PointerInput),
        /// Moves piled up; take them from the sink.
        Moved,
        Escape,
    }

    struct Sink {
        events: UnboundedSender<Event>,
        held_at: POINT,
        moved: (f32, f32),
    }

    impl Sink {
        fn send(&mut self, body: Body) {
            // Moves go first, so a press lands where the pointer was.
            if let Some(moved) = self.take_moved() {
                let _ = self.events.send(Event::Input(moved));
            }
            let _ = self
                .events
                .send(Event::Input(PointerInput { body: Some(body) }));
        }

        fn take_moved(&mut self) -> Option<PointerInput> {
            let (dx, dy) = std::mem::take(&mut self.moved);
            (dx != 0.0 || dy != 0.0).then_some(PointerInput {
                body: Some(Body::Move(PointerMove { dx, dy })),
            })
        }
    }

    /// Set while the pointer is on the phone; the hooks swallow input only then.
    static SINK: Mutex<Option<Sink>> = Mutex::new(None);

    struct Screens {
        left: i32,
        top: i32,
        width: i32,
        height: i32,
    }

    fn screens() -> Screens {
        unsafe {
            Screens {
                left: GetSystemMetrics(SM_XVIRTUALSCREEN),
                top: GetSystemMetrics(SM_YVIRTUALSCREEN),
                width: GetSystemMetrics(SM_CXVIRTUALSCREEN),
                height: GetSystemMetrics(SM_CYVIRTUALSCREEN),
            }
        }
    }

    fn pressed(key: VIRTUAL_KEY) -> bool {
        unsafe { GetAsyncKeyState(i32::from(key.0)) < 0 }
    }

    pub(super) fn watch(app: AppHandle, side: PhoneSide) {
        std::thread::spawn(move || {
            // Only crosses after the pointer has been away from the edge, so it doesn't bounce
            // straight back after returning or after the phone said no.
            let mut armed = true;
            loop {
                std::thread::sleep(POLL);
                let Some(side) = *side.0.lock() else {
                    continue;
                };
                let mut at = POINT::default();
                if unsafe { GetCursorPos(&mut at) }.is_err() {
                    continue;
                }
                let screens = screens();
                let at_edge = match side {
                    Side::Left => at.x <= screens.left,
                    Side::Right => at.x >= screens.left + screens.width - 1,
                };
                if !at_edge {
                    armed = true;
                    continue;
                }
                // Dragging a window to the edge isn't a move to the phone.
                if !armed || pressed(VK_LBUTTON) {
                    continue;
                }
                armed = false;
                let y = (at.y - screens.top) as f32 / screens.height as f32;
                tauri::async_runtime::block_on(cross(&app, side, at, y));
            }
        });
    }

    async fn cross(app: &AppHandle, side: Side, at: POINT, y: f32) {
        let device = app.state::<DesktopRuntimeState>().device.clone();
        let Some(mux) = device
            .sessions
            .connected()
            .first()
            .and_then(|peer| device.sessions.get(peer))
        else {
            return;
        };
        let start = PointerStart {
            y,
            from_left: side == Side::Right,
        };
        let (mut control, mut leaves) = match mux.point(start).await {
            Ok(Some(pointer)) => pointer,
            Ok(None) => {
                let _ = app.emit("pointer-unavailable", ());
                return;
            }
            Err(error) => return tracing::warn!("Couldn't move the pointer to the phone: {error}"),
        };

        let screens = screens();
        let held_at = POINT {
            x: match side {
                Side::Left => screens.left + HOLD_INSET,
                Side::Right => screens.left + screens.width - 1 - HOLD_INSET,
            },
            y: at.y.clamp(
                screens.top + HOLD_INSET,
                screens.top + screens.height - HOLD_INSET,
            ),
        };
        let _ = unsafe { SetCursorPos(held_at.x, held_at.y) };
        let (events, mut queued) = unbounded_channel();
        let hooks = Hooks::install(Sink {
            events,
            held_at,
            moved: (0.0, 0.0),
        });

        let back_at = loop {
            tokio::select! {
                event = queued.recv() => {
                    let input = match event {
                        Some(Event::Input(input)) => Some(input),
                        Some(Event::Moved) => SINK.lock().as_mut().and_then(Sink::take_moved),
                        Some(Event::Escape) | None => break y,
                    };
                    if let Some(input) = input {
                        if control.send(&input).await.is_err() {
                            break y;
                        }
                    }
                }
                left = leaves.next() => break left.map_or(y, |left| left.y),
            }
        };
        drop(hooks);
        drop(control);
        // Just inside the edge, where it went over on the phone.
        let x = match side {
            Side::Left => screens.left + 2,
            Side::Right => screens.left + screens.width - 3,
        };
        let y = screens.top + (back_at.clamp(0.0, 1.0) * (screens.height - 1) as f32) as i32;
        let _ = unsafe { SetCursorPos(x, y) };
    }

    /// Low-level mouse and keyboard hooks on their own thread, which needs a message loop.
    struct Hooks {
        thread_id: u32,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    impl Hooks {
        fn install(sink: Sink) -> Self {
            *SINK.lock() = Some(sink);
            let (ready, id) = std::sync::mpsc::channel();
            let thread = std::thread::spawn(move || unsafe {
                let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), None, 0);
                let keys = SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_hook), None, 0);
                let _ = ready.send(GetCurrentThreadId());
                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
                for hook in [mouse, keys].into_iter().flatten() {
                    let _ = UnhookWindowsHookEx(hook);
                }
            });
            Self {
                thread_id: id.recv().unwrap_or_default(),
                thread: Some(thread),
            }
        }
    }

    impl Drop for Hooks {
        fn drop(&mut self) {
            *SINK.lock() = None;
            let _ = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            if let Some(sink) = SINK.lock().as_mut() {
                let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
                match wparam.0 as u32 {
                    // The pointer is held still, so where Windows would put it is the move.
                    WM_MOUSEMOVE => {
                        let first = sink.moved == (0.0, 0.0);
                        sink.moved.0 += (info.pt.x - sink.held_at.x) as f32;
                        sink.moved.1 += (info.pt.y - sink.held_at.y) as f32;
                        if first {
                            let _ = sink.events.send(Event::Moved);
                        }
                    }
                    WM_LBUTTONDOWN => sink.send(Body::Press(true)),
                    WM_LBUTTONUP => sink.send(Body::Press(false)),
                    WM_RBUTTONUP => sink.send(button(ScreenButton::Back)),
                    WM_MBUTTONUP => sink.send(button(ScreenButton::Home)),
                    WM_MOUSEWHEEL => {
                        let notches = f32::from((info.mouseData >> 16) as i16) / 120.0;
                        sink.send(Body::Scroll(notches));
                    }
                    _ => {}
                }
                return LRESULT(1);
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    unsafe extern "system" fn key_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            if let Some(sink) = SINK.lock().as_mut() {
                let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                if matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN) {
                    match VIRTUAL_KEY(info.vkCode as u16) {
                        VK_ESCAPE => {
                            let _ = sink.events.send(Event::Escape);
                        }
                        VK_BACK => sink.send(key(ScreenKey::Backspace)),
                        VK_RETURN => sink.send(key(ScreenKey::Enter)),
                        vk => {
                            if let Some(text) = typed(vk, info.scanCode) {
                                sink.send(screen(screen_input::Body::Text(text)));
                            }
                        }
                    }
                }
                return LRESULT(1);
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    /// The text a key makes with the keyboard layout and Shift, Caps Lock and AltGr as they are.
    /// Shortcuts held with Ctrl alone make none.
    fn typed(vk: VIRTUAL_KEY, scan: u32) -> Option<String> {
        if pressed(VK_CONTROL) && !pressed(VK_MENU) {
            return None;
        }
        let mut state = [0u8; 256];
        for modifier in [VK_SHIFT, VK_CONTROL, VK_MENU] {
            if pressed(modifier) {
                state[usize::from(modifier.0)] = 0x80;
            }
        }
        state[usize::from(VK_CAPITAL.0)] =
            (unsafe { GetKeyState(i32::from(VK_CAPITAL.0)) } & 1) as u8;
        let mut text = [0u16; 8];
        // Flag 4 leaves the layout's dead-key state alone, so typing here doesn't disturb it.
        let length = unsafe { ToUnicode(u32::from(vk.0), scan, Some(&state), &mut text, 4) };
        let text = String::from_utf16(text.get(..usize::try_from(length).ok()?)?).ok()?;
        (!text.is_empty() && !text.chars().any(char::is_control)).then_some(text)
    }

    pub(super) fn inject(input: PointerInput, remainder: &mut (f32, f32)) {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
            MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL,
        };
        match input.body {
            Some(Body::Move(moved)) => {
                let (x, y) = (remainder.0 + moved.dx, remainder.1 + moved.dy);
                let (dx, dy) = (x.trunc(), y.trunc());
                *remainder = (x - dx, y - dy);
                mouse(dx as i32, dy as i32, 0, MOUSEEVENTF_MOVE);
            }
            Some(Body::Press(down)) => {
                mouse(
                    0,
                    0,
                    0,
                    if down {
                        MOUSEEVENTF_LEFTDOWN
                    } else {
                        MOUSEEVENTF_LEFTUP
                    },
                );
            }
            Some(Body::PressSecondary(down)) => {
                mouse(
                    0,
                    0,
                    0,
                    if down {
                        MOUSEEVENTF_RIGHTDOWN
                    } else {
                        MOUSEEVENTF_RIGHTUP
                    },
                );
            }
            Some(Body::Scroll(notches)) => mouse(0, 0, (notches * 120.0) as i32, MOUSEEVENTF_WHEEL),
            Some(Body::Screen(screen)) => match screen.body {
                Some(screen_input::Body::Text(text)) => {
                    crate::actions::type_text(&text);
                }
                Some(screen_input::Body::Key(key)) => match ScreenKey::try_from(key) {
                    Ok(ScreenKey::Backspace) => press_key(VK_BACK),
                    Ok(ScreenKey::Enter) => press_key(VK_RETURN),
                    Err(_) => {}
                },
                _ => {}
            },
            None => {}
        }
    }

    pub(super) fn release_buttons() {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTUP,
        };
        mouse(0, 0, 0, MOUSEEVENTF_LEFTUP | MOUSEEVENTF_RIGHTUP);
    }

    fn mouse(
        dx: i32,
        dy: i32,
        wheel: i32,
        flags: windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS,
    ) {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEINPUT,
        };
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: wheel as u32,
                    dwFlags: flags,
                    ..Default::default()
                },
            },
        };
        unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    }

    fn press_key(vk: VIRTUAL_KEY) {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        };
        let key = |up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        Default::default()
                    },
                    ..Default::default()
                },
            },
        };
        unsafe {
            SendInput(
                &[key(false), key(true)],
                std::mem::size_of::<INPUT>() as i32,
            )
        };
    }

    fn screen(body: screen_input::Body) -> Body {
        Body::Screen(ScreenInput { body: Some(body) })
    }

    fn button(button: ScreenButton) -> Body {
        screen(screen_input::Body::Button(button.into()))
    }

    fn key(key: ScreenKey) -> Body {
        screen(screen_input::Body::Key(key.into()))
    }
}
