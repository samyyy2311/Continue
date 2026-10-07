// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

// Generated UniFFI scaffolding (uniffi 0.28) leaves blank lines after doc comments.
#![allow(clippy::empty_line_after_doc_comments)]

uniffi::include_scaffolding!("continue");

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use thiserror::Error;

use device::{Device, PairError, Stores};
use history::{Direction, Kind};
use identity::FileSecretStore;
use pairing::{DeviceKeys, TrustedPeer};
use protocol::CapabilityId;
use sessions::SessionState;
use tokio::runtime::Runtime;
use transport::TransportCertificate;

#[derive(Debug, Error)]
pub enum ContinueFfiError {
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Invalid QR payload: {0}")]
    InvalidQr(String),

    #[error("Pairing failed: {0}")]
    PairingFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Core has not been initialized")]
    NotInitialized,
}

fn internal(error: impl ToString) -> ContinueFfiError {
    ContinueFfiError::InternalError(error.to_string())
}

fn database(error: impl ToString) -> ContinueFfiError {
    ContinueFfiError::DatabaseError(error.to_string())
}

impl From<device::DeviceError> for ContinueFfiError {
    fn from(error: device::DeviceError) -> Self {
        match error {
            device::DeviceError::Signer(_) => internal(error),
            _ => database(error),
        }
    }
}

/// What the app's secret store reports when it can't do what was asked.
#[derive(Debug, Error)]
pub enum SecretStoreFfiError {
    #[error("{reason}")]
    Unavailable { reason: String },
}

impl From<uniffi::UnexpectedUniFFICallbackError> for SecretStoreFfiError {
    fn from(error: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::Unavailable {
            reason: error.reason,
        }
    }
}

/// The platform's secret store, implemented by the app.
pub trait SecretStoreFfi: Send + Sync {
    fn store(&self, label: String, secret: Vec<u8>) -> Result<(), SecretStoreFfiError>;
    fn load(&self, label: String) -> Result<Option<Vec<u8>>, SecretStoreFfiError>;
    fn delete(&self, label: String) -> Result<(), SecretStoreFfiError>;
}

/// Set by the app before `init_core`; the keys stay in files until it is.
static KEY_STORE: Mutex<Option<Arc<dyn SecretStoreFfi>>> = Mutex::new(None);

pub fn set_key_store(store: Box<dyn SecretStoreFfi>) {
    *KEY_STORE.lock().unwrap() = Some(Arc::from(store));
}

pub struct PhotoFfi {
    pub id: String,
    pub name: String,
    pub taken_at: u64,
    pub thumbnail: Vec<u8>,
}

pub trait PhotoLibraryFfi: Send + Sync {
    fn recent(&self, limit: u32) -> Option<Vec<PhotoFfi>>;
    fn file(&self, id: String) -> Option<String>;
}

static PHOTO_LIBRARY: Mutex<Option<Arc<dyn sessions::PhotoLibrary>>> = Mutex::new(None);

pub fn set_photo_library(library: Box<dyn PhotoLibraryFfi>) {
    *PHOTO_LIBRARY.lock().unwrap() = Some(Arc::new(AppPhotos(library)));
}

struct AppPhotos(Box<dyn PhotoLibraryFfi>);

impl From<PhotoFfi> for protocol::v1::Photo {
    fn from(photo: PhotoFfi) -> Self {
        Self {
            id: photo.id,
            name: photo.name,
            taken_at: photo.taken_at,
            thumbnail: photo.thumbnail,
        }
    }
}

impl sessions::PhotoLibrary for AppPhotos {
    fn recent(&self, limit: u32) -> Option<Vec<protocol::v1::Photo>> {
        Some(self.0.recent(limit)?.into_iter().map(Into::into).collect())
    }

    fn file(&self, id: &str) -> Option<PathBuf> {
        self.0.file(id.to_string()).map(PathBuf::from)
    }
}

pub struct ConversationFfi {
    pub id: String,
    pub address: String,
    pub name: String,
    pub snippet: String,
    pub at: u64,
    pub unread: bool,
}

pub struct TextMessageFfi {
    pub id: String,
    pub body: String,
    pub at: u64,
    pub outgoing: bool,
}

pub struct ContactFfi {
    pub name: String,
    pub number: String,
    pub favorite: bool,
    pub photo: Vec<u8>,
}

pub trait MessageStoreFfi: Send + Sync {
    fn conversations(&self, limit: u32) -> Option<Vec<ConversationFfi>>;
    fn conversation(&self, id: String, limit: u32) -> Option<Vec<TextMessageFfi>>;
    fn send(&self, address: String, body: String) -> bool;
    fn contacts(&self, limit: u32) -> Option<Vec<ContactFfi>>;
}

pub struct NowPlayingFfi {
    pub title: String,
    pub artist: String,
    pub app: String,
    pub playing: bool,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub art: Option<Vec<u8>>,
}

pub enum MediaCommandFfi {
    PlayPause,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
}

pub trait RingerFfi: Send + Sync {
    fn ring(&self, on: bool) -> bool;
}

static RINGER: Mutex<Option<Arc<dyn sessions::Ringer>>> = Mutex::new(None);

pub fn set_ringer(ringer: Box<dyn RingerFfi>) {
    *RINGER.lock().unwrap() = Some(Arc::new(AppRinger(ringer)));
}

struct AppRinger(Box<dyn RingerFfi>);

impl sessions::Ringer for AppRinger {
    fn ring(&self, on: bool) -> bool {
        self.0.ring(on)
    }
}

pub trait MediaControlFfi: Send + Sync {
    fn command(&self, command: MediaCommandFfi) -> bool;
}

static MEDIA_CONTROL: Mutex<Option<Arc<dyn sessions::MediaControl>>> = Mutex::new(None);

pub fn set_media_control(control: Box<dyn MediaControlFfi>) {
    *MEDIA_CONTROL.lock().unwrap() = Some(Arc::new(AppMedia(control)));
}

struct AppMedia(Box<dyn MediaControlFfi>);

impl sessions::MediaControl for AppMedia {
    fn command(&self, command: protocol::v1::media_command::Kind) -> bool {
        use protocol::v1::media_command::Kind;
        self.0.command(match command {
            Kind::PlayPause => MediaCommandFfi::PlayPause,
            Kind::Next => MediaCommandFfi::Next,
            Kind::Previous => MediaCommandFfi::Previous,
            Kind::VolumeUp => MediaCommandFfi::VolumeUp,
            Kind::VolumeDown => MediaCommandFfi::VolumeDown,
        })
    }
}

pub enum CallStateFfi {
    Ringing,
    Talking,
    Ended,
}

pub struct CallFfi {
    pub state: CallStateFfi,
    pub number: String,
    pub name: String,
}

pub trait CallControlFfi: Send + Sync {
    fn answer(&self) -> bool;
    fn decline(&self) -> bool;
    fn silence(&self) -> bool;
    fn dial(&self, number: String) -> bool;
}

static CALL_CONTROL: Mutex<Option<Arc<dyn sessions::CallControl>>> = Mutex::new(None);

pub fn set_call_control(control: Box<dyn CallControlFfi>) {
    *CALL_CONTROL.lock().unwrap() = Some(Arc::new(AppCalls(control)));
}

struct AppCalls(Box<dyn CallControlFfi>);

impl sessions::CallControl for AppCalls {
    fn answer(&self) -> bool {
        self.0.answer()
    }

    fn decline(&self) -> bool {
        self.0.decline()
    }

    fn silence(&self) -> bool {
        self.0.silence()
    }

    fn dial(&self, number: &str) -> bool {
        self.0.dial(number.to_string())
    }
}

pub enum VideoKindFfi {
    Screen,
    Camera,
}

pub struct VideoStartFfi {
    pub width: u32,
    pub height: u32,
    pub rotation: u32,
}

impl From<VideoStartFfi> for protocol::v1::VideoStart {
    fn from(start: VideoStartFfi) -> Self {
        Self {
            started: true,
            width: start.width,
            height: start.height,
            rotation: start.rotation,
        }
    }
}

pub enum CameraControlFfi {
    Front { front: bool },
    Framing { on: bool },
}

pub trait CameraSourceFfi: Send + Sync {
    fn start(&self, max_size: u32, front: bool) -> Option<VideoStartFfi>;
    fn control(&self, control: CameraControlFfi);
    fn stop(&self);
}

static CAMERA_SOURCE: Mutex<Option<Arc<sessions::CameraSource>>> = Mutex::new(None);

pub fn set_camera_source(source: Box<dyn CameraSourceFfi>) {
    *CAMERA_SOURCE.lock().unwrap() = Some(Arc::new(AppCamera(source)));
}

struct AppCamera(Box<dyn CameraSourceFfi>);

impl sessions::VideoSource<protocol::v1::CameraRequest, protocol::v1::CameraControl> for AppCamera {
    fn start(&self, request: protocol::v1::CameraRequest) -> Option<protocol::v1::VideoStart> {
        self.0
            .start(request.max_size, request.front)
            .map(Into::into)
    }

    fn control(&self, control: protocol::v1::CameraControl) {
        use protocol::v1::camera_control::Body;
        let control = match control.body {
            Some(Body::Front(front)) => CameraControlFfi::Front { front },
            Some(Body::Framing(on)) => CameraControlFfi::Framing { on },
            None => return,
        };
        self.0.control(control);
    }

    fn stop(&self) {
        self.0.stop();
    }
}

pub enum TouchActionFfi {
    Down,
    Move,
    Up,
}

pub enum ScreenInputFfi {
    Touch {
        action: TouchActionFfi,
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

pub trait ScreenSourceFfi: Send + Sync {
    fn start(&self, max_size: u32) -> Option<VideoStartFfi>;
    fn input(&self, input: ScreenInputFfi);
    fn stop(&self);
}

static SCREEN_SOURCE: Mutex<Option<Arc<sessions::ScreenSource>>> = Mutex::new(None);

pub fn set_screen_source(source: Box<dyn ScreenSourceFfi>) {
    *SCREEN_SOURCE.lock().unwrap() = Some(Arc::new(AppScreen(source)));
}

struct AppScreen(Box<dyn ScreenSourceFfi>);

impl sessions::VideoSource<protocol::v1::ScreenRequest, protocol::v1::ScreenInput> for AppScreen {
    fn start(&self, request: protocol::v1::ScreenRequest) -> Option<protocol::v1::VideoStart> {
        self.0.start(request.max_size).map(Into::into)
    }

    fn control(&self, input: protocol::v1::ScreenInput) {
        if let Some(input) = screen_input(input) {
            self.0.input(input);
        }
    }

    fn stop(&self) {
        self.0.stop();
    }
}

fn screen_input(input: protocol::v1::ScreenInput) -> Option<ScreenInputFfi> {
    use protocol::v1::{screen_input::Body, touch::Action, ScreenButton, ScreenKey};
    Some(match input.body? {
        Body::Touch(touch) => ScreenInputFfi::Touch {
            action: match touch.action() {
                Action::Down => TouchActionFfi::Down,
                Action::Move => TouchActionFfi::Move,
                Action::Up => TouchActionFfi::Up,
            },
            x: touch.x,
            y: touch.y,
        },
        Body::Swipe(swipe) => ScreenInputFfi::Swipe {
            from_x: swipe.from_x,
            from_y: swipe.from_y,
            to_x: swipe.to_x,
            to_y: swipe.to_y,
            duration_ms: swipe.duration_ms,
        },
        Body::Button(button) => match ScreenButton::try_from(button).ok()? {
            ScreenButton::Back => ScreenInputFfi::Back,
            ScreenButton::Home => ScreenInputFfi::Home,
            ScreenButton::Recents => ScreenInputFfi::Recents,
        },
        Body::Text(text) => ScreenInputFfi::Text { text },
        Body::Key(key) => match ScreenKey::try_from(key).ok()? {
            ScreenKey::Backspace => ScreenInputFfi::Backspace,
            ScreenKey::Enter => ScreenInputFfi::Enter,
        },
    })
}

pub enum PointerInputFfi {
    Move { dx: f32, dy: f32 },
    Press { down: bool },
    PressSecondary { down: bool },
    Scroll { notches: f32 },
    Screen { input: ScreenInputFfi },
}

pub trait PointerTargetFfi: Send + Sync {
    fn start(&self, y: f32, from_left: bool) -> bool;
    fn input(&self, input: PointerInputFfi);
    fn stop(&self);
}

static POINTER_TARGET: Mutex<Option<Arc<dyn sessions::PointerTarget>>> = Mutex::new(None);

/// Set while a computer's pointer is on the phone.
static POINTER_LEAVE: Mutex<Option<sessions::PointerLeave>> = Mutex::new(None);

pub fn set_pointer_target(target: Box<dyn PointerTargetFfi>) {
    *POINTER_TARGET.lock().unwrap() = Some(Arc::new(AppPointer(target)));
}

/// The phone used as a touchpad for a computer: input goes there until stopped.
static TOUCHPAD: Mutex<Option<sessions::PointerControl>> = Mutex::new(None);

/// False when the computer can't take input, such as with that switched off there.
pub fn touchpad_start(peer_fingerprint: String) -> Result<bool, ContinueFfiError> {
    let (runtime, device) = device()?;
    let mux = device
        .sessions
        .get(&peer_fingerprint)
        .ok_or_else(|| internal("Not connected"))?;
    let start = protocol::v1::PointerStart::default();
    let control = runtime
        .block_on(mux.point(start))
        .map_err(internal)?
        .map(|(control, _)| control);
    let started = control.is_some();
    *TOUCHPAD.lock().unwrap() = control;
    Ok(started)
}

/// False once the computer has gone.
pub fn touchpad_input(input: PointerInputFfi) -> bool {
    use protocol::v1::pointer_input::Body;
    let body = match input {
        PointerInputFfi::Move { dx, dy } => Body::Move(protocol::v1::PointerMove { dx, dy }),
        PointerInputFfi::Press { down } => Body::Press(down),
        PointerInputFfi::PressSecondary { down } => Body::PressSecondary(down),
        PointerInputFfi::Scroll { notches } => Body::Scroll(notches),
        PointerInputFfi::Screen { input } => Body::Screen(screen_input_message(input)),
    };
    let Ok((runtime, _)) = device() else {
        return false;
    };
    let mut touchpad = TOUCHPAD.lock().unwrap();
    let Some(control) = touchpad.as_mut() else {
        return false;
    };
    let input = protocol::v1::PointerInput { body: Some(body) };
    runtime.block_on(control.send(&input)).is_ok()
}

pub fn touchpad_stop() {
    TOUCHPAD.lock().unwrap().take();
}

fn screen_input_message(input: ScreenInputFfi) -> protocol::v1::ScreenInput {
    use protocol::v1::{screen_input::Body, touch::Action, ScreenButton, ScreenKey, Swipe, Touch};
    let body = match input {
        ScreenInputFfi::Touch { action, x, y } => Body::Touch(Touch {
            action: match action {
                TouchActionFfi::Down => Action::Down,
                TouchActionFfi::Move => Action::Move,
                TouchActionFfi::Up => Action::Up,
            }
            .into(),
            x,
            y,
        }),
        ScreenInputFfi::Swipe {
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
        } => Body::Swipe(Swipe {
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
        }),
        ScreenInputFfi::Back => Body::Button(ScreenButton::Back.into()),
        ScreenInputFfi::Home => Body::Button(ScreenButton::Home.into()),
        ScreenInputFfi::Recents => Body::Button(ScreenButton::Recents.into()),
        ScreenInputFfi::Text { text } => Body::Text(text),
        ScreenInputFfi::Backspace => Body::Key(ScreenKey::Backspace.into()),
        ScreenInputFfi::Enter => Body::Key(ScreenKey::Enter.into()),
    };
    protocol::v1::ScreenInput { body: Some(body) }
}

/// The app calls this when the pointer goes back over the edge it came in by.
pub fn pointer_left(y: f32) {
    if let Some(leave) = POINTER_LEAVE.lock().unwrap().as_ref() {
        leave(y);
    }
}

struct AppPointer(Box<dyn PointerTargetFfi>);

impl sessions::PointerTarget for AppPointer {
    fn start(&self, start: protocol::v1::PointerStart, leave: sessions::PointerLeave) -> bool {
        *POINTER_LEAVE.lock().unwrap() = Some(leave);
        self.0.start(start.y, start.from_left)
    }

    fn input(&self, input: protocol::v1::PointerInput) {
        use protocol::v1::pointer_input::Body;
        let input = match input.body {
            Some(Body::Move(moved)) => PointerInputFfi::Move {
                dx: moved.dx,
                dy: moved.dy,
            },
            Some(Body::Press(down)) => PointerInputFfi::Press { down },
            Some(Body::PressSecondary(down)) => PointerInputFfi::PressSecondary { down },
            Some(Body::Scroll(notches)) => PointerInputFfi::Scroll { notches },
            Some(Body::Screen(screen)) => match screen_input(screen) {
                Some(input) => PointerInputFfi::Screen { input },
                None => return,
            },
            None => return,
        };
        self.0.input(input);
    }

    fn stop(&self) {
        *POINTER_LEAVE.lock().unwrap() = None;
        self.0.stop();
    }
}

fn video_feed(kind: VideoKindFfi) -> Option<sessions::VideoFeed> {
    let core = CORE.lock().unwrap();
    let core = core.as_ref()?;
    Some(match kind {
        VideoKindFfi::Screen => core.screen_feed.clone(),
        VideoKindFfi::Camera => core.camera_feed.clone(),
    })
}

pub fn push_video_frame(kind: VideoKindFfi, data: Vec<u8>, key: bool, rotation: u32) -> bool {
    let frame = protocol::v1::VideoFrame {
        data,
        key,
        rotation,
        ..Default::default()
    };
    video_feed(kind).is_some_and(|feed| feed.push(frame))
}

/// 16-bit stereo PCM at 48 kHz, what's playing while the screen is shared.
pub fn push_audio(kind: VideoKindFfi, pcm: Vec<u8>) -> bool {
    video_feed(kind).is_some_and(|feed| feed.push_audio(pcm))
}

pub fn end_video(kind: VideoKindFfi) {
    if let Some(feed) = video_feed(kind) {
        feed.end();
    }
}

static SHARED_FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn set_shared_folder(path: String) {
    *SHARED_FOLDER.lock().unwrap() = Some(PathBuf::from(path));
}

static MESSAGE_STORE: Mutex<Option<Arc<dyn sessions::MessageStore>>> = Mutex::new(None);

pub enum SearchKindFfi {
    File,
    Text,
    Contact,
}

pub struct SearchResultFfi {
    pub kind: SearchKindFfi,
    pub title: String,
    pub detail: String,
    pub reference: String,
    pub at: u64,
}

pub trait PhoneSearchFfi: Send + Sync {
    fn search(&self, query: String, limit: u32) -> Option<Vec<SearchResultFfi>>;
}

static PHONE_SEARCH: Mutex<Option<Arc<dyn sessions::PhoneSearch>>> = Mutex::new(None);

pub fn set_phone_search(search: Box<dyn PhoneSearchFfi>) {
    *PHONE_SEARCH.lock().unwrap() = Some(Arc::new(AppSearch(search)));
}

struct AppSearch(Box<dyn PhoneSearchFfi>);

impl sessions::PhoneSearch for AppSearch {
    fn search(&self, query: &str, limit: u32) -> Option<Vec<protocol::v1::SearchResult>> {
        use protocol::v1::search_result::Kind;
        let results = self.0.search(query.to_string(), limit)?;
        Some(
            results
                .into_iter()
                .map(|result| protocol::v1::SearchResult {
                    kind: match result.kind {
                        SearchKindFfi::File => Kind::File,
                        SearchKindFfi::Text => Kind::Text,
                        SearchKindFfi::Contact => Kind::Contact,
                    }
                    .into(),
                    title: result.title,
                    detail: result.detail,
                    reference: result.reference,
                    at: result.at,
                })
                .collect(),
        )
    }
}

pub fn set_message_store(store: Box<dyn MessageStoreFfi>) {
    *MESSAGE_STORE.lock().unwrap() = Some(Arc::new(AppMessages(store)));
}

struct AppMessages(Box<dyn MessageStoreFfi>);

impl sessions::MessageStore for AppMessages {
    fn conversations(&self, limit: u32) -> Option<Vec<protocol::v1::Conversation>> {
        let found = self.0.conversations(limit)?.into_iter();
        Some(
            found
                .map(|c| protocol::v1::Conversation {
                    id: c.id,
                    address: c.address,
                    name: c.name,
                    snippet: c.snippet,
                    at: c.at,
                    unread: c.unread,
                })
                .collect(),
        )
    }

    fn conversation(&self, id: &str, limit: u32) -> Option<Vec<protocol::v1::TextMessage>> {
        let found = self.0.conversation(id.to_string(), limit)?.into_iter();
        Some(
            found
                .map(|t| protocol::v1::TextMessage {
                    id: t.id,
                    body: t.body,
                    at: t.at,
                    outgoing: t.outgoing,
                })
                .collect(),
        )
    }

    fn send(&self, address: &str, body: &str) -> bool {
        self.0.send(address.to_string(), body.to_string())
    }

    fn contacts(&self, limit: u32) -> Option<Vec<protocol::v1::Contact>> {
        let found = self.0.contacts(limit)?.into_iter();
        Some(
            found
                .map(|c| protocol::v1::Contact {
                    name: c.name,
                    number: c.number,
                    favorite: c.favorite,
                    photo: c.photo,
                })
                .collect(),
        )
    }
}

/// The app's store, seen as the core's `SecretStore`.
struct AppSecretStore(Arc<dyn SecretStoreFfi>);

fn from_app(error: SecretStoreFfiError) -> identity::SecretStoreError {
    identity::SecretStoreError::Unavailable(error.to_string())
}

impl identity::SecretStore for AppSecretStore {
    fn store(
        &self,
        label: &str,
        secret: zeroize::Zeroizing<Vec<u8>>,
    ) -> Result<(), identity::SecretStoreError> {
        self.0
            .store(label.to_string(), secret.to_vec())
            .map_err(from_app)
    }

    fn load(
        &self,
        label: &str,
    ) -> Result<Option<zeroize::Zeroizing<Vec<u8>>>, identity::SecretStoreError> {
        Ok(self
            .0
            .load(label.to_string())
            .map_err(from_app)?
            .map(zeroize::Zeroizing::new))
    }

    fn delete(&self, label: &str) -> Result<(), identity::SecretStoreError> {
        self.0.delete(label.to_string()).map_err(from_app)
    }
}

struct CoreState {
    runtime: Arc<Runtime>,
    device: Device,
    listener: quinn::Endpoint,
    discovery_tasks: Vec<tokio::task::JoinHandle<()>>,
    incoming: sessions::IncomingFiles,
    /// The name paired devices see for this phone.
    this_device: sessions::ThisDevice,
    screen_feed: sessions::VideoFeed,
    camera_feed: sessions::VideoFeed,
    /// Copies of files kept for a computer that isn't connected.
    waiting_dir: PathBuf,
}

static CORE: Mutex<Option<CoreState>> = Mutex::new(None);

fn with_core<T>(
    f: impl FnOnce(&mut CoreState) -> Result<T, ContinueFfiError>,
) -> Result<T, ContinueFfiError> {
    f(CORE
        .lock()
        .unwrap()
        .as_mut()
        .ok_or(ContinueFfiError::NotInitialized)?)
}

/// The device and the runtime it runs on, for calls that wait without holding up the rest.
fn device() -> Result<(Arc<Runtime>, Device), ContinueFfiError> {
    with_core(|core| Ok((core.runtime.clone(), core.device.clone())))
}

/// A file or text a paired device sent to this phone. Files arrive as `file_path` and
/// `file_name`, text as `text`.
pub struct ReceivedFfi {
    /// The history entry, for `set_history_location` once the app has moved the file.
    pub history_id: Option<i64>,
    pub peer_fingerprint: String,
    pub peer_name: String,
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub size: u64,
    pub text: Option<String>,
    /// A PNG to put on the clipboard.
    pub image: Option<Vec<u8>>,
}

/// Items for the app to pick up whenever it next asks. Kept outside `CORE` so waiting on one
/// never holds up other calls.
struct Inbox<T> {
    items: Mutex<VecDeque<T>>,
    arrived: Condvar,
}

impl<T> Inbox<T> {
    const fn new() -> Self {
        Self {
            items: Mutex::new(VecDeque::new()),
            arrived: Condvar::new(),
        }
    }

    /// Adds an item, dropping the oldest beyond `limit` so an app that stops listening
    /// doesn't grow the queue forever.
    fn push(&self, item: T, limit: usize) {
        let mut items = self.items.lock().unwrap();
        items.push_back(item);
        while items.len() > limit {
            items.pop_front();
        }
        self.arrived.notify_all();
    }

    fn next(&self, timeout: Duration) -> Option<T> {
        let items = self.items.lock().unwrap();
        let (mut items, _) = self
            .arrived
            .wait_timeout_while(items, timeout, |items| items.is_empty())
            .unwrap();
        items.pop_front()
    }
}

static RECEIVED: Inbox<ReceivedFfi> = Inbox::new();
const RECEIVED_LIMIT: usize = 100;

/// What a computer did with one of this phone's notifications.
pub enum NotificationEventFfi {
    Action {
        notification_id: String,
        action_id: String,
        reply_text: String,
    },
    Dismiss {
        notification_id: String,
    },
    Mute {
        package_name: String,
    },
}

static NOTIFICATION_EVENTS: Inbox<NotificationEventFfi> = Inbox::new();

/// The wallpaper each computer last sent, by fingerprint.
static PEER_WALLPAPERS: Mutex<BTreeMap<String, Vec<u8>>> = Mutex::new(BTreeMap::new());

/// Saves received files and text to history and hands them to the app through
/// `next_received`.
fn deliver_received(
    mut handlers: sessions::SessionCapabilityHandlers,
    stores: Stores,
) -> sessions::SessionCapabilityHandlers {
    let names = stores.clone();
    handlers.on_device_info = Some(Arc::new(move |peer, device| {
        names.rename_peer(peer, &device.name);
    }));
    let files = stores.clone();
    handlers.on_file_received = Some(Arc::new(move |peer, file| {
        // The app moves the file on, then sets its location.
        let saved = files.received_file(peer, &file, None);
        RECEIVED.push(
            ReceivedFfi {
                history_id: saved.history_id,
                peer_fingerprint: peer.to_string(),
                peer_name: saved.peer_name,
                file_path: Some(file.path.to_string_lossy().into_owned()),
                file_name: Some(file.file_name),
                size: file.bytes_received,
                text: None,
                image: None,
            },
            RECEIVED_LIMIT,
        );
    }));
    handlers.on_clipboard_received = Some(Arc::new(move |peer, update| {
        if update.format() == protocol::v1::ClipboardFormat::ImagePng {
            RECEIVED.push(
                ReceivedFfi {
                    history_id: None,
                    peer_fingerprint: peer.to_string(),
                    peer_name: stores.peer_name(peer),
                    file_path: None,
                    file_name: None,
                    size: update.payload.len() as u64,
                    text: None,
                    image: Some(update.payload),
                },
                RECEIVED_LIMIT,
            );
            return;
        }
        let text = String::from_utf8_lossy(&update.payload).into_owned();
        let saved = stores.received_text(peer, &text);
        RECEIVED.push(
            ReceivedFfi {
                history_id: saved.history_id,
                peer_fingerprint: peer.to_string(),
                peer_name: saved.peer_name,
                file_path: None,
                file_name: None,
                size: update.payload.len() as u64,
                text: Some(text),
                image: None,
            },
            RECEIVED_LIMIT,
        );
    }));
    handlers.on_device_look = Some(Arc::new(|peer, look| {
        let mut wallpapers = PEER_WALLPAPERS.lock().unwrap();
        if look.wallpaper.is_empty() {
            wallpapers.remove(peer);
        } else {
            wallpapers.insert(peer.to_string(), look.wallpaper);
        }
    }));
    handlers.on_notification = Some(Arc::new(|_peer, body| {
        let event = match body {
            notifications::Body::Action(action) => NotificationEventFfi::Action {
                notification_id: action.notification_id,
                action_id: action.action_id,
                reply_text: action.reply_text,
            },
            notifications::Body::Dismiss(dismiss) => NotificationEventFfi::Dismiss {
                notification_id: dismiss.notification_id,
            },
            notifications::Body::Mute(mute) => NotificationEventFfi::Mute {
                package_name: mute.package_name,
            },
            // The phone shows its own notifications, not a computer's.
            notifications::Body::Post(_) => return,
        };
        NOTIFICATION_EVENTS.push(event, RECEIVED_LIMIT);
    }));
    handlers
}

pub struct HistoryEntryFfi {
    pub id: i64,
    /// Unix time in milliseconds.
    pub at: u64,
    pub received: bool,
    pub is_text: bool,
    pub label: String,
    pub peer_fingerprint: String,
    pub peer_name: String,
    pub size: u64,
    pub failed: bool,
    pub location: Option<String>,
}

fn history() -> Result<history::HistoryStore, ContinueFfiError> {
    with_core(|core| Ok(core.device.stores.history.clone()))
}

/// What this phone sent and received, newest first.
pub fn list_history(limit: u32) -> Result<Vec<HistoryEntryFfi>, ContinueFfiError> {
    let entries = history()?.list(limit).map_err(database)?;
    Ok(entries
        .into_iter()
        .map(|entry| HistoryEntryFfi {
            id: entry.id,
            at: entry.at,
            received: entry.item.direction == Direction::Received,
            is_text: entry.item.kind == Kind::Text,
            label: entry.item.label,
            peer_fingerprint: entry.item.peer_fingerprint,
            peer_name: entry.item.peer_name,
            size: entry.item.size,
            failed: entry.item.failed,
            location: entry.item.location,
        })
        .collect())
}

pub fn clear_history() -> Result<(), ContinueFfiError> {
    history()?.clear().map_err(database)
}

/// Notes where the app put a received file.
pub fn set_history_location(id: i64, location: String) -> Result<(), ContinueFfiError> {
    history()?.set_location(id, &location).map_err(database)
}

/// Waits up to `timeout_ms` for the next file or text a paired device sent.
pub fn next_received(timeout_ms: u32) -> Option<ReceivedFfi> {
    RECEIVED.next(Duration::from_millis(timeout_ms.into()))
}

pub struct TrustedPeerFfi {
    pub fingerprint: String,
    pub display_name: String,
    pub paired_at: u64,
}

impl From<TrustedPeer> for TrustedPeerFfi {
    fn from(p: TrustedPeer) -> Self {
        Self {
            fingerprint: p.fingerprint,
            display_name: p.display_name,
            paired_at: p.paired_at,
        }
    }
}

pub fn init_core(db_path: String) -> Result<(), ContinueFfiError> {
    // Shut the previous core down first so its listener gives up the port.
    let previous = CORE.lock().unwrap().take();
    if let Some(mut previous) = previous {
        stop_tasks(&mut previous.discovery_tasks);
        previous.listener.close(0u32.into(), b"restarting");
    }

    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(internal)?,
    );
    let stores = Stores::open(&db_path)?;
    let keys = load_device_keys(&db_path)?;

    let db_dir = Path::new(&db_path)
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();
    // Next to the database, in the app's own storage; the app moves files on from there.
    let download_dir = db_dir.join("received");
    let _ = std::fs::create_dir_all(&download_dir);
    let incoming = sessions::IncomingFiles::default();
    // Named by `set_device_name` once the app has read the phone's name.
    let this_device = sessions::ThisDevice::default();
    let mut handlers = deliver_received(
        sessions::SessionCapabilityHandlers::new(download_dir)
            .with_this_device(this_device.clone())
            .with_incoming(incoming.clone()),
        stores.clone(),
    );
    handlers.photo_library = PHOTO_LIBRARY.lock().unwrap().clone();
    handlers.message_store = MESSAGE_STORE.lock().unwrap().clone();
    handlers.phone_search = PHONE_SEARCH.lock().unwrap().clone();
    handlers.shared_folder = SHARED_FOLDER.lock().unwrap().clone();
    handlers.call_control = CALL_CONTROL.lock().unwrap().clone();
    handlers.media_control = MEDIA_CONTROL.lock().unwrap().clone();
    handlers.ringer = RINGER.lock().unwrap().clone();
    handlers.pointer_target = POINTER_TARGET.lock().unwrap().clone();
    handlers.screen_source = SCREEN_SOURCE.lock().unwrap().clone();
    handlers.camera_source = CAMERA_SOURCE.lock().unwrap().clone();
    let (screen_feed, camera_feed) = (handlers.screen_feed.clone(), handlers.camera_feed.clone());
    let waiting_dir = db_dir.join("waiting");
    let device = Device::new(
        stores,
        keys,
        handlers,
        Some(send_waiting_on_connect(waiting_dir.clone())),
    )?;
    let listener = {
        let _runtime = runtime.enter();
        device
            .listen()
            .map_err(|e| internal(format!("Could not listen: {e}")))?
    };

    *CORE.lock().unwrap() = Some(CoreState {
        runtime,
        device,
        listener,
        discovery_tasks: Vec::new(),
        incoming,
        this_device,
        screen_feed,
        camera_feed,
        waiting_dir,
    });
    Ok(())
}

/// Keys live in the app's secret store (the Android Keystore) so pairings survive a restart.
/// Keys from earlier versions, in a `secrets` folder beside the database, move there; without
/// a store they stay in that folder. An in-memory database (used by tests) gets throwaway keys.
fn load_device_keys(db_path: &str) -> Result<DeviceKeys, ContinueFfiError> {
    if db_path == ":memory:" {
        let seed = crypto::keys::generate_ed25519_seed();
        let signing_key = crypto::keys::signing_key_from_seed(&seed.0);
        return Ok(DeviceKeys {
            identity_signer: Arc::new(identity::InMemorySigner::new(signing_key)),
            transport_cert: Arc::new(TransportCertificate::generate().map_err(internal)?),
        });
    }

    let secrets_dir = Path::new(db_path)
        .parent()
        .ok_or_else(|| internal(format!("Database path has no folder: {db_path}")))?
        .join("secrets");
    let files = FileSecretStore::new(secrets_dir).map_err(internal)?;
    let app_store = KEY_STORE.lock().unwrap().clone();
    match app_store {
        Some(app_store) => DeviceKeys::load_or_create(&identity::PlatformFirstStore::new(
            Box::new(AppSecretStore(app_store)),
            files,
        )),
        None => DeviceKeys::load_or_create(&files),
    }
    .map_err(internal)
}

/// Sets the name paired devices see for this phone, from the next session on.
pub fn set_device_name(name: String) -> Result<(), ContinueFfiError> {
    with_core(|core| {
        core.this_device.set_name(&name);
        Ok(())
    })
}

pub struct DeviceStatusFfi {
    pub battery_percent: u32,
    pub charging: bool,
    pub cell_bars: Option<u32>,
    pub wifi_bars: Option<u32>,
    pub carrier: String,
    pub network: String,
}

pub fn set_device_status(status: DeviceStatusFfi) -> Result<(), ContinueFfiError> {
    let (runtime, device) = device()?;
    let _runtime = runtime.enter();
    device.sessions.report_status(protocol::v1::DeviceStatus {
        battery_percent: status.battery_percent.min(100),
        charging: status.charging,
        cell_bars: status.cell_bars.map(|bars| bars.min(MAX_BARS)),
        wifi_bars: status.wifi_bars.map(|bars| bars.min(MAX_BARS)),
        carrier: status.carrier,
        network: status.network,
    });
    Ok(())
}

const MAX_BARS: u32 = 4;

/// `wallpaper_color` is 0xRRGGBB; `wallpaper` is a small JPEG when the app can read it.
pub fn set_look(wallpaper_color: u32, wallpaper: Option<Vec<u8>>) -> Result<(), ContinueFfiError> {
    let (runtime, device) = device()?;
    let _runtime = runtime.enter();
    device.sessions.report_look(protocol::v1::DeviceLook {
        wallpaper_color: wallpaper_color & 0xff_ffff,
        wallpaper: wallpaper.unwrap_or_default(),
    });
    Ok(())
}

pub fn peer_wallpaper(peer_fingerprint: String) -> Option<Vec<u8>> {
    PEER_WALLPAPERS
        .lock()
        .unwrap()
        .get(&peer_fingerprint)
        .cloned()
}

pub fn get_device_fingerprint() -> Result<String, ContinueFfiError> {
    with_core(|core| Ok(core.device.fingerprint.clone()))
}

/// A file on its way in.
pub struct IncomingFileFfi {
    pub transfer_id: String,
    pub peer_name: String,
    pub file_name: String,
    pub received: u64,
    pub total: u64,
}

/// Files coming in right now, oldest first, for the app to show while they arrive.
pub fn list_incoming() -> Vec<IncomingFileFfi> {
    with_core(|core| {
        Ok(core
            .incoming
            .list()
            .into_iter()
            .map(|file| IncomingFileFfi {
                peer_name: core.device.stores.peer_name(&file.peer),
                transfer_id: file.transfer_id,
                file_name: file.file_name,
                received: file.received,
                total: file.total,
            })
            .collect())
    })
    .unwrap_or_default()
}

/// Stops a file part way. False if it already finished or never started.
pub fn cancel_incoming(transfer_id: String) -> bool {
    with_core(|core| Ok(core.incoming.cancel(&transfer_id))).unwrap_or(false)
}

pub fn get_device_spki_hash() -> Result<String, ContinueFfiError> {
    with_core(|core| Ok(hex::encode(core.device.keys.transport_cert.spki_hash)))
}

/// Advertises this device's listener and connects to paired devices as they appear on
/// the network, until `stop_discovery`.
pub fn start_discovery(protocol_version: u32) -> Result<(), ContinueFfiError> {
    with_core(|core| {
        let port = core.listener.local_addr().map_err(internal)?.port();
        stop_tasks(&mut core.discovery_tasks);
        let _runtime = core.runtime.enter();
        core.discovery_tasks = core.device.discover(port, protocol_version).into();
        Ok(())
    })
}

pub fn stop_discovery() -> Result<(), ContinueFfiError> {
    with_core(|core| {
        stop_tasks(&mut core.discovery_tasks);
        Ok(())
    })
}

fn stop_tasks(tasks: &mut Vec<tokio::task::JoinHandle<()>>) {
    for task in tasks.drain(..) {
        task.abort();
    }
}

fn pairing_failed(error: impl ToString) -> ContinueFfiError {
    ContinueFfiError::PairingFailed(error.to_string())
}

pub fn pair_from_qr(qr_payload: String) -> Result<TrustedPeerFfi, ContinueFfiError> {
    let (runtime, device) = device()?;
    match runtime.block_on(device.pair_with_code(&qr_payload)) {
        Ok(peer) => Ok(peer.into()),
        Err(PairError::BadCode(reason)) => Err(ContinueFfiError::InvalidQr(reason)),
        Err(error) => Err(pairing_failed(error)),
    }
}

pub struct NearbyComputerFfi {
    pub name: String,
    pub code: String,
}

/// Computers showing a pairing code on this network, gathered for `wait_ms`.
pub fn nearby_computers(wait_ms: u32) -> Result<Vec<NearbyComputerFfi>, ContinueFfiError> {
    let (runtime, _) = device()?;
    let wait = std::time::Duration::from_millis(u64::from(wait_ms));
    let found = runtime
        .block_on(discovery::find_nearby(wait))
        .map_err(internal)?;
    Ok(found
        .into_iter()
        .map(|computer| NearbyComputerFfi {
            name: computer.name,
            code: computer.code,
        })
        .collect())
}

/// A nearby pairing waiting for the person to compare codes.
static PENDING_PAIR: Mutex<Option<device::PendingPair>> = Mutex::new(None);

/// Pairs with a computer found nearby. Returns the six digits to compare with its screen;
/// nothing is trusted until [`confirm_nearby_pairing`] accepts.
pub fn pair_nearby(code: String) -> Result<String, ContinueFfiError> {
    let (runtime, device) = device()?;
    let pending = match runtime.block_on(device.pair_nearby(&code)) {
        Ok(pending) => pending,
        Err(PairError::BadCode(reason)) => return Err(ContinueFfiError::InvalidQr(reason)),
        Err(error) => return Err(pairing_failed(error)),
    };
    let digits = pending.code().to_string();
    *PENDING_PAIR.lock().unwrap() = Some(pending);
    Ok(digits)
}

/// The computer, once accepted; None when the codes didn't match and it was turned down.
pub fn confirm_nearby_pairing(accept: bool) -> Result<Option<TrustedPeerFfi>, ContinueFfiError> {
    let Some(pending) = PENDING_PAIR.lock().unwrap().take() else {
        return Ok(None);
    };
    if !accept {
        return Ok(None);
    }
    let peer = pending.accept().map_err(pairing_failed)?;
    Ok(Some(peer.into()))
}

pub fn list_trusted_peers() -> Result<Vec<TrustedPeerFfi>, ContinueFfiError> {
    let peers = with_core(|core| core.device.stores.trust.list_peers().map_err(database))?;
    Ok(peers.into_iter().map(Into::into).collect())
}

pub fn remove_trusted_peer(fingerprint: String) -> Result<bool, ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime
        .block_on(device.forget(&fingerprint))
        .map_err(database)
}

pub fn is_allowed(peer_fingerprint: String, capability_id: u32) -> Result<bool, ContinueFfiError> {
    with_core(|core| {
        Ok(core
            .device
            .stores
            .permissions
            .is_allowed(&peer_fingerprint, CapabilityId(capability_id)))
    })
}

pub fn set_allowed(
    peer_fingerprint: String,
    capability_id: u32,
    allowed: bool,
) -> Result<(), ContinueFfiError> {
    with_core(|core| {
        core.device
            .stores
            .permissions
            .set_allowed(&peer_fingerprint, CapabilityId(capability_id), allowed)
            .map_err(database)
    })
}

pub fn connect_to_peer(peer_fingerprint: String, endpoint: String) -> Result<(), ContinueFfiError> {
    let addr: std::net::SocketAddr = endpoint
        .parse()
        .map_err(|e| internal(format!("Invalid endpoint address: {e}")))?;
    let (runtime, device) = device()?;
    match runtime.block_on(device.connect(&peer_fingerprint, addr)) {
        Ok(()) => Ok(()),
        Err(device::ConnectError::Store(error)) => Err(database(error)),
        Err(error) => Err(internal(error)),
    }
}

pub fn is_peer_connected(peer_fingerprint: String) -> Result<bool, ContinueFfiError> {
    with_core(|core| Ok(core.device.sessions.state(&peer_fingerprint) == SessionState::Connected))
}

pub fn reconnect(peer_fingerprint: String) -> Result<(), ContinueFfiError> {
    with_core(|core| {
        core.device.sessions.resume_auto_connect(&peer_fingerprint);
        Ok(())
    })
}

/// Pausing drops every connection and turns new ones away until resumed.
pub fn set_paused(paused: bool) -> Result<(), ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime.block_on(device.sessions.set_paused(paused));
    Ok(())
}

pub fn disconnect(peer_fingerprint: String) -> Result<(), ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime.block_on(device.sessions.disconnect(&peer_fingerprint));
    Ok(())
}

/// The file is the app's temporary copy, so history keeps its name but not its place. If the
/// connection drops part way, this waits for it to come back and sends the rest.
pub fn send_file(peer_fingerprint: String, file_path: String) -> Result<u64, ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime
        .block_on(device.send_file(
            &peer_fingerprint,
            Path::new(&file_path),
            None,
            None::<fn(u64, u64)>,
        ))
        .map_err(internal)
}

pub struct NotificationActionFfi {
    pub action_id: String,
    pub label: String,
    pub is_reply: bool,
}

pub struct NotificationFfi {
    pub notification_id: String,
    pub package_name: String,
    pub app_name: String,
    pub title: String,
    pub body: String,
    pub timestamp: u64,
    pub actions: Vec<NotificationActionFfi>,
}

/// Sends `body` to every connected computer in the background, so the caller never waits
/// on the network.
fn forward(body: notifications::Body) -> Result<(), ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime.spawn(async move {
        for peer in device.sessions.connected() {
            if let Err(error) = device.send_notification(&peer, body.clone()).await {
                tracing::warn!("Couldn't forward a notification to {peer}: {error}");
            }
        }
    });
    Ok(())
}

/// Shows one of this phone's notifications on connected computers.
pub fn forward_notification(notification: NotificationFfi) -> Result<(), ContinueFfiError> {
    forward(notifications::Body::Post(notifications::NotificationPost {
        notification_id: notification.notification_id,
        package_name: notification.package_name,
        app_name: notification.app_name,
        title: notification.title,
        body: notification.body,
        timestamp: notification.timestamp,
        actions: notification
            .actions
            .into_iter()
            .map(|action| notifications::NotificationAction {
                action_id: action.action_id,
                label: action.label,
                is_reply: action.is_reply,
            })
            .collect(),
    }))
}

/// Takes a notification that went away on the phone off connected computers too.
pub fn forward_notification_removed(notification_id: String) -> Result<(), ContinueFfiError> {
    forward(notifications::Body::Dismiss(
        notifications::NotificationDismiss {
            notification_id,
            package_name: String::new(),
        },
    ))
}

/// Waits up to `timeout_ms` for a computer to reply to or dismiss one of this phone's
/// notifications.
pub fn next_notification_event(timeout_ms: u32) -> Option<NotificationEventFfi> {
    NOTIFICATION_EVENTS.next(Duration::from_millis(timeout_ms.into()))
}

pub fn announce_photo(photo: PhotoFfi) -> Result<(), ContinueFfiError> {
    let body = protocol::v1::photos_message::Body::Taken(photo.into());
    tell_connected(move |device, peer| {
        let body = body.clone();
        async move { device.photos(&peer, body).await.map(drop) }
    })
}

pub fn announce_messages_changed() -> Result<(), ContinueFfiError> {
    let body = protocol::v1::messages_message::Body::Changed(Default::default());
    tell_connected(move |device, peer| {
        let body = body.clone();
        async move { device.messages(&peer, body).await.map(drop) }
    })
}

pub fn announce_call(call: CallFfi) -> Result<(), ContinueFfiError> {
    use protocol::v1::call::State;
    let state = match call.state {
        CallStateFfi::Ringing => State::Ringing,
        CallStateFfi::Talking => State::Talking,
        CallStateFfi::Ended => State::Ended,
    };
    let body = protocol::v1::calls_message::Body::Call(protocol::v1::Call {
        state: state.into(),
        number: call.number,
        name: call.name,
    });
    tell_connected(move |device, peer| {
        let body = body.clone();
        async move { device.calls(&peer, body).await.map(drop) }
    })
}

pub fn announce_now_playing(playing: NowPlayingFfi) -> Result<(), ContinueFfiError> {
    let body = protocol::v1::media_message::Body::NowPlaying(protocol::v1::NowPlaying {
        title: playing.title,
        artist: playing.artist,
        app: playing.app,
        playing: playing.playing,
        duration_ms: playing.duration_ms,
        position_ms: playing.position_ms,
        art: playing.art.unwrap_or_default(),
    });
    tell_connected(move |device, peer| {
        let body = body.clone();
        async move { device.media(&peer, body).await.map(drop) }
    })
}

/// Best effort: failures are logged, not returned.
fn tell_connected<F>(send: impl Fn(Device, String) -> F) -> Result<(), ContinueFfiError>
where
    F: std::future::Future<Output = Result<(), device::SendError>>,
{
    let (runtime, device) = device()?;
    runtime.block_on(async {
        for peer in device.sessions.connected() {
            if let Err(error) = send(device.clone(), peer.clone()).await {
                tracing::debug!("Couldn't reach {peer}: {error}");
            }
        }
    });
    Ok(())
}

/// Kept until the computer connects. The file is the app's temporary copy; it's moved into
/// the core's own folder and deleted once sent.
pub fn send_file_later(
    peer_fingerprint: String,
    file_path: String,
) -> Result<(), ContinueFfiError> {
    let (_, device) = device()?;
    let from = Path::new(&file_path);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    let folder = waiting_dir()?.join(now.map_or(0, |since| since.as_nanos()).to_string());
    let to = folder.join(from.file_name().unwrap_or_default());
    std::fs::create_dir_all(&folder)
        .and_then(|()| std::fs::rename(from, &to).or_else(|_| std::fs::copy(from, &to).map(drop)))
        .map_err(internal)?;
    device
        .send_later(
            &peer_fingerprint,
            history::Kind::File,
            &to.to_string_lossy(),
        )
        .map(drop)
        .map_err(internal)
}

pub fn send_text_later(peer_fingerprint: String, text: String) -> Result<(), ContinueFfiError> {
    let (_, device) = device()?;
    device
        .send_later(&peer_fingerprint, history::Kind::Text, &text)
        .map(drop)
        .map_err(internal)
}

/// Sends what was kept for a computer as soon as it connects, then deletes the copies made
/// for it in `waiting_dir`.
fn send_waiting_on_connect(waiting_dir: PathBuf) -> sessions::StateListener {
    Arc::new(move |peer, state| {
        if state != SessionState::Connected {
            return;
        }
        // Only missing before `init_core` finishes, when nothing has connected yet.
        let Ok((runtime, device)) = device() else {
            return;
        };
        let (peer, waiting_dir) = (peer.to_string(), waiting_dir.clone());
        runtime.spawn(async move {
            device.share_snippets(&peer).await;
            device
                .send_waiting(&peer, |item, _| {
                    let folder = Path::new(&item.content).parent();
                    if let Some(folder) = folder.filter(|f| f.starts_with(&waiting_dir)) {
                        let _ = std::fs::remove_dir_all(folder);
                    }
                })
                .await;
        });
    })
}

fn waiting_dir() -> Result<PathBuf, ContinueFfiError> {
    with_core(|core| Ok(core.waiting_dir.clone()))
}

pub fn send_clipboard_image(
    peer_fingerprint: String,
    png: Vec<u8>,
) -> Result<(), ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime
        .block_on(device.send_image(&peer_fingerprint, png))
        .map_err(internal)
}

pub struct SnippetFfi {
    pub id: String,
    pub text: String,
}

impl From<history::Snippet> for SnippetFfi {
    fn from(snippet: history::Snippet) -> Self {
        Self {
            id: snippet.id,
            text: snippet.text,
        }
    }
}

/// What the phone broadcasts over Bluetooth right now so paired computers know it's near.
pub fn presence_token() -> Result<Vec<u8>, ContinueFfiError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    with_core(|core| Ok(pairing::presence_token(&core.device.fingerprint, now).to_vec()))
}

/// The four words the computer shows for this pairing too.
pub fn pairing_words(peer_fingerprint: String) -> Result<String, ContinueFfiError> {
    with_core(|core| {
        Ok(pairing::pairing_words(&core.device.fingerprint, &peer_fingerprint).join(" "))
    })
}

/// Pinned text, newest first.
pub fn snippets() -> Result<Vec<SnippetFfi>, ContinueFfiError> {
    let (_, device) = device()?;
    let snippets = device.stores.history.snippets(false).map_err(internal)?;
    Ok(snippets.into_iter().map(Into::into).collect())
}

/// Pins text on every paired computer too.
pub fn pin_snippet(text: String) -> Result<SnippetFfi, ContinueFfiError> {
    let (runtime, device) = device()?;
    let snippet = runtime.block_on(device.pin(&text)).map_err(internal)?;
    Ok(snippet.into())
}

pub fn unpin_snippet(id: String) -> Result<(), ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime.block_on(device.unpin(&id)).map_err(internal)
}

pub enum ComputerActionFfi {
    Lock,
    Sleep,
    TypeText { text: String },
    OpenLink { url: String },
}

/// Rings the computer so it can be found, or stops it. False if it didn't.
pub fn ring_computer(peer_fingerprint: String, on: bool) -> Result<bool, ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime
        .block_on(device.ring(&peer_fingerprint, on))
        .map_err(internal)
}

/// False if the computer didn't do it, such as with that switched off there.
pub fn act_on_computer(
    peer_fingerprint: String,
    action: ComputerActionFfi,
) -> Result<bool, ContinueFfiError> {
    use protocol::v1::computer_action::Body;
    let body = match action {
        ComputerActionFfi::Lock => Body::Lock(true),
        ComputerActionFfi::Sleep => Body::Sleep(true),
        ComputerActionFfi::TypeText { text } => Body::TypeText(text),
        ComputerActionFfi::OpenLink { url } => Body::OpenLink(url),
    };
    let (runtime, device) = device()?;
    runtime
        .block_on(device.act(&peer_fingerprint, body))
        .map_err(internal)
}

pub fn send_clipboard_text(peer_fingerprint: String, text: String) -> Result<(), ContinueFfiError> {
    let (runtime, device) = device()?;
    runtime
        .block_on(device.send_text(&peer_fingerprint, text))
        .map_err(internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stands in for the Android Keystore.
    #[derive(Default)]
    struct MemoryStore(Mutex<std::collections::HashMap<String, Vec<u8>>>);

    impl SecretStoreFfi for Arc<MemoryStore> {
        fn store(&self, label: String, secret: Vec<u8>) -> Result<(), SecretStoreFfiError> {
            self.0.lock().unwrap().insert(label, secret);
            Ok(())
        }
        fn load(&self, label: String) -> Result<Option<Vec<u8>>, SecretStoreFfiError> {
            Ok(self.0.lock().unwrap().get(&label).cloned())
        }
        fn delete(&self, label: String) -> Result<(), SecretStoreFfiError> {
            self.0.lock().unwrap().remove(&label);
            Ok(())
        }
    }

    // One test, since the key store is shared by the whole process.
    #[test]
    fn keys_move_from_files_into_the_apps_store() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("continue-ffi-keys-{nanos}"));
        let db = dir.join("continue.db").to_string_lossy().into_owned();
        std::fs::create_dir_all(&dir).unwrap();

        // An earlier version, with no store set.
        let before = load_device_keys(&db).unwrap();
        let key = |keys: &DeviceKeys| keys.identity_signer.verifying_key().unwrap();

        let app_store = Arc::new(MemoryStore::default());
        set_key_store(Box::new(app_store.clone()));
        let after = load_device_keys(&db).unwrap();
        *KEY_STORE.lock().unwrap() = None;

        assert_eq!(key(&after), key(&before));
        assert_eq!(
            after.transport_cert.spki_hash,
            before.transport_cert.spki_hash
        );
        let saved = app_store.0.lock().unwrap();
        assert!(saved.contains_key("device_identity") && saved.contains_key("transport_cert"));
        let files = std::fs::read_dir(dir.join("secrets"))
            .unwrap()
            .filter(|entry| entry.as_ref().unwrap().path().extension() == Some("secret".as_ref()))
            .count();
        assert_eq!(files, 0, "no key left in files");

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn memory_stores() -> Stores {
        Stores {
            trust: pairing::TrustStore::in_memory().unwrap(),
            permissions: Arc::new(permissions::PermissionStore::in_memory().unwrap()),
            history: history::HistoryStore::in_memory().unwrap(),
        }
    }

    #[test]
    fn received_files_and_text_reach_the_app_with_their_sender() {
        let stores = memory_stores();
        let history = stores.history.clone();
        let handlers = deliver_received(
            sessions::SessionCapabilityHandlers::new(std::env::temp_dir()),
            stores,
        );
        (handlers.on_file_received.unwrap())(
            "laptop",
            transfer::ReceivedFile {
                transfer_id: "tx-1".to_string(),
                path: "/data/received/report.pdf".into(),
                file_name: "report.pdf".to_string(),
                bytes_received: 2048,
            },
        );

        let file = next_received(100).expect("the file");
        assert_eq!(file.peer_fingerprint, "laptop");
        assert_eq!(file.file_name.as_deref(), Some("report.pdf"));
        assert_eq!(file.size, 2048);
        assert!(file.text.is_none());
        assert!(next_received(10).is_none());

        let saved = history.list(10).unwrap();
        assert_eq!(file.history_id, Some(saved[0].id));
        assert_eq!(saved[0].item.direction, Direction::Received);
        assert_eq!(saved[0].item.label, "report.pdf");
    }
}
