// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Notify;
use tracing::{debug, error, info};

use capabilities::CapabilityQuery;
use clipboard::{ClipboardAck, ClipboardFormat, ClipboardUpdate};
use notifications::Body as NotificationBody;
use protocol::v1::Photo;
use protocol::CapabilityId;
use transfer::{receive_file, send_file, ReceivedFile};
use transport::{read_msg, write_msg, TransportError};

use crate::actions::{serve_actions, ComputerActions};
use crate::calls::{serve_calls, CallControl};
use crate::device::{PeerDevice, ThisDevice};
use crate::files::serve_files;
use crate::find::{serve_ring, Ringer};
use crate::incoming::{IncomingFiles, SaveFolder};
use crate::media::{serve_media, MediaControl};
use crate::messages::{serve_messages, MessageStore};
use crate::multiplexer::{OnPeerUpdate, PeerUpdate, SessionMultiplexer};
use crate::photos::{serve_photos, PhotoLibrary};
use crate::pointer::{serve_pointer, PointerTarget};
use crate::search::{serve_search, PhoneSearch};
use crate::snippets::{serve_snippets, SnippetStore};
use crate::video::{serve_video, CameraSource, ScreenSource, VideoFeed};

/// Called with the sender's fingerprint and what it sent.
pub type OnReceived<T> = Arc<dyn Fn(&str, T) + Send + Sync>;

/// Callbacks and configuration for active capabilities over a multiplexed session.
#[derive(Clone)]
pub struct SessionCapabilityHandlers {
    pub save_folder: SaveFolder,
    pub incoming: IncomingFiles,
    pub on_file_received: Option<OnReceived<ReceivedFile>>,
    pub on_clipboard_received: Option<OnReceived<ClipboardUpdate>>,
    /// Called with notifications the peer shows here, and with replies to and dismissals of
    /// the ones this device sent it.
    pub on_notification: Option<OnReceived<NotificationBody>>,
    /// How this device introduces itself to peers.
    pub this_device: ThisDevice,
    /// Called with what a peer says about itself each time a session starts.
    pub on_device_info: Option<OnReceived<PeerDevice>>,
    /// Called with the peer's battery when a session starts and each time it changes.
    pub on_device_status: Option<OnReceived<protocol::v1::DeviceStatus>>,
    pub on_device_look: Option<OnReceived<protocol::v1::DeviceLook>>,
    pub photo_library: Option<Arc<dyn PhotoLibrary>>,
    pub on_photo_taken: Option<OnReceived<Photo>>,
    pub message_store: Option<Arc<dyn MessageStore>>,
    pub on_messages_changed: Option<OnReceived<()>>,
    pub shared_folder: Option<PathBuf>,
    pub call_control: Option<Arc<dyn CallControl>>,
    pub on_call: Option<OnReceived<protocol::v1::Call>>,
    pub media_control: Option<Arc<dyn MediaControl>>,
    pub on_now_playing: Option<OnReceived<protocol::v1::NowPlaying>>,
    pub ringer: Option<Arc<dyn Ringer>>,
    pub screen_source: Option<Arc<ScreenSource>>,
    pub screen_feed: VideoFeed,
    pub camera_source: Option<Arc<CameraSource>>,
    pub camera_feed: VideoFeed,
    pub pointer_target: Option<Arc<dyn PointerTarget>>,
    pub computer_actions: Option<Arc<dyn ComputerActions>>,
    pub phone_search: Option<Arc<dyn PhoneSearch>>,
    /// Set by the device, which keeps snippets in its store.
    pub snippet_store: Option<Arc<dyn SnippetStore>>,
    /// Called when a peer's snippets changed the ones here.
    pub on_snippets_changed: Option<OnReceived<()>>,
    /// None means everything is allowed.
    pub permission_store: Option<Arc<permissions::PermissionStore>>,
}

impl SessionCapabilityHandlers {
    pub fn new(save_folder: impl Into<PathBuf>) -> Self {
        Self {
            save_folder: SaveFolder::new(save_folder),
            incoming: IncomingFiles::default(),
            on_file_received: None,
            on_clipboard_received: None,
            on_notification: None,
            this_device: ThisDevice::default(),
            on_device_info: None,
            on_device_status: None,
            on_device_look: None,
            photo_library: None,
            on_photo_taken: None,
            message_store: None,
            on_messages_changed: None,
            shared_folder: None,
            call_control: None,
            on_call: None,
            media_control: None,
            on_now_playing: None,
            ringer: None,
            screen_source: None,
            screen_feed: VideoFeed::default(),
            camera_source: None,
            camera_feed: VideoFeed::default(),
            pointer_target: None,
            computer_actions: None,
            snippet_store: None,
            phone_search: None,
            on_snippets_changed: None,
            permission_store: None,
        }
    }

    pub fn with_permission_store(mut self, store: Arc<permissions::PermissionStore>) -> Self {
        self.permission_store = Some(store);
        self
    }

    /// Reports files as they come in, to whoever holds a clone of `incoming`.
    pub fn with_incoming(mut self, incoming: IncomingFiles) -> Self {
        self.incoming = incoming;
        self
    }

    pub fn with_this_device(mut self, device: ThisDevice) -> Self {
        self.this_device = device;
        self
    }
}

pub(crate) fn allowed(
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    capability: CapabilityId,
) -> bool {
    handlers
        .permission_store
        .as_ref()
        .is_none_or(|store| store.is_allowed(peer, capability))
}

/// For blocking app callbacks and filesystem calls. None if the closure panicked.
pub(crate) async fn off_runtime<T: Send + 'static>(
    f: impl FnOnce() -> Option<T> + Send + 'static,
) -> Option<T> {
    tokio::task::spawn_blocking(f).await.ok().flatten()
}

pub(crate) async fn send_requested_file(mux: &SessionMultiplexer, peer: &str, path: &Path) {
    let since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let transfer_id = format!("asked-{}", since_epoch.as_millis());
    if let Err(e) = mux
        .send_file_to_peer(path, transfer_id, None::<fn(u64, u64)>)
        .await
    {
        error!("Couldn't send {} to {peer}: {e}", path.display());
    }
}

/// Rejects out-of-range values so apps can show them as they are.
fn plausible(status: &protocol::v1::DeviceStatus) -> bool {
    let bars = |bars: Option<u32>| bars.is_none_or(|bars| bars <= 4);
    status.battery_percent <= 100 && bars(status.cell_bars) && bars(status.wifi_bars)
}

/// Spawns the capability router loop processing incoming streams dispatched by the multiplexer.
pub fn spawn_capabilities_dispatcher(
    mux: Arc<SessionMultiplexer>,
    handlers: SessionCapabilityHandlers,
    buffer_size: usize,
) {
    let peer_fingerprint = mux.peer_fingerprint().to_string();
    let on_update: OnPeerUpdate = {
        let (peer, on_info, on_status, on_look) = (
            peer_fingerprint.clone(),
            handlers.on_device_info.clone(),
            handlers.on_device_status.clone(),
            handlers.on_device_look.clone(),
        );
        Arc::new(move |update| match update {
            PeerUpdate::Info(info) => {
                if let (Some(on_info), Some(device)) = (&on_info, PeerDevice::from_info(&info)) {
                    on_info(&peer, device);
                }
            }
            PeerUpdate::Status(status) if plausible(&status) => {
                if let Some(on_status) = &on_status {
                    on_status(&peer, status);
                }
            }
            PeerUpdate::Status(_) => {}
            PeerUpdate::Look(look) => {
                if let Some(on_look) = &on_look {
                    on_look(&peer, look);
                }
            }
        })
    };
    let mut stream_rx = mux.spawn_router_with(buffer_size, Some(on_update));
    let clipboard = mux.clipboard().clone();

    tokio::spawn(async move {
        while let Some(mut stream) = stream_rx.recv().await {
            let mux = mux.clone();
            let handlers = handlers.clone();
            let peer_fp = peer_fingerprint.clone();
            let clipboard = clipboard.clone();

            tokio::spawn(async move {
                match stream.capability {
                    CapabilityId::FILE_TRANSFER => {
                        debug!("Handling incoming file transfer stream from {peer_fp}");
                        let (handlers_ref, peer_ref) = (&handlers, &peer_fp);
                        let allows_files =
                            allowed(handlers_ref, peer_ref, CapabilityId::FILE_TRANSFER);
                        let check =
                            |_: &protocol::v1::FileTransferRequest| async move { allows_files };

                        // Signalled by `IncomingFiles::cancel`.
                        let stop = Arc::new(Notify::new());
                        let progress = |req: &protocol::v1::FileTransferRequest, received| {
                            handlers_ref.incoming.update(peer_ref, req, received, &stop);
                        };
                        let stopped = {
                            let stop = stop.clone();
                            async move { stop.notified().await }
                        };
                        let result = receive_file(
                            &mut stream.send_stream,
                            &mut stream.recv_stream,
                            &handlers.save_folder.get(),
                            Some(check),
                            Some(progress),
                            stopped,
                        )
                        .await;
                        match result {
                            Ok(received) => {
                                info!(
                                    "Successfully received file {} ({} bytes) from {peer_fp}",
                                    received.file_name, received.bytes_received
                                );
                                if let Some(cb) = &handlers.on_file_received {
                                    cb(&peer_fp, received);
                                }
                            }
                            Err(e) => {
                                error!("Failed to receive file from {peer_fp}: {e}");
                            }
                        }
                        handlers.incoming.end(&stop);
                    }
                    CapabilityId::CLIPBOARD => {
                        debug!("Handling incoming clipboard stream from {peer_fp}");
                        let query = CapabilityQuery::negotiated(
                            CapabilityId::CLIPBOARD,
                            allowed(&handlers, &peer_fp, CapabilityId::CLIPBOARD),
                        );

                        let on_received = handlers.on_clipboard_received.clone();
                        let result = clipboard
                            .receive_update(
                                &mut stream.send_stream,
                                &mut stream.recv_stream,
                                &query,
                                |format, payload| {
                                    debug!(
                                        "Applying clipboard update format {format:?}, size {}",
                                        payload.len()
                                    );
                                    Ok(())
                                },
                            )
                            .await;

                        match result {
                            Ok(update) => {
                                if let Some(cb) = on_received {
                                    cb(&peer_fp, update);
                                }
                            }
                            Err(e) => {
                                error!("Failed to process clipboard update from {peer_fp}: {e}");
                            }
                        }
                    }
                    CapabilityId::NOTIFICATIONS => {
                        let body = match notifications::read(&mut stream.recv_stream).await {
                            Ok(body) => body,
                            Err(e) => {
                                error!("Couldn't read a notification from {peer_fp}: {e}");
                                return;
                            }
                        };
                        // Replies, dismissals and mutes only act on notifications this device sent.
                        let shown = match &body {
                            NotificationBody::Post(_) => {
                                allowed(&handlers, &peer_fp, CapabilityId::NOTIFICATIONS)
                            }
                            NotificationBody::Action(_)
                            | NotificationBody::Dismiss(_)
                            | NotificationBody::Mute(_) => true,
                        };
                        if let (true, Some(cb)) = (shown, &handlers.on_notification) {
                            cb(&peer_fp, body);
                        }
                        let _ = notifications::acknowledge(&mut stream.send_stream, shown).await;
                    }
                    CapabilityId::PHOTOS => {
                        if let Err(e) = serve_photos(&mux, &handlers, &peer_fp, stream).await {
                            error!("Couldn't answer a photos request from {peer_fp}: {e}");
                        }
                    }
                    CapabilityId::SCREEN => {
                        let source = handlers
                            .screen_source
                            .clone()
                            .filter(|_| allowed(&handlers, &peer_fp, CapabilityId::SCREEN));
                        if let Err(e) = serve_video(source, &handlers.screen_feed, stream).await {
                            debug!("Stopped sharing the screen with {peer_fp}: {e}");
                        }
                    }
                    CapabilityId::CAMERA => {
                        let source = handlers
                            .camera_source
                            .clone()
                            .filter(|_| allowed(&handlers, &peer_fp, CapabilityId::CAMERA));
                        if let Err(e) = serve_video(source, &handlers.camera_feed, stream).await {
                            debug!("Stopped sharing the camera with {peer_fp}: {e}");
                        }
                    }
                    CapabilityId::POINTER => {
                        if let Err(e) = serve_pointer(&handlers, &peer_fp, stream).await {
                            debug!("Stopped taking pointer input from {peer_fp}: {e}");
                        }
                    }
                    CapabilityId::SEARCH => {
                        if let Err(e) = serve_search(&handlers, &peer_fp, stream).await {
                            error!("Couldn't answer a search from {peer_fp}: {e}");
                        }
                    }
                    CapabilityId::SNIPPETS => {
                        if let Err(e) = serve_snippets(&handlers, &peer_fp, stream).await {
                            error!("Couldn't take snippets from {peer_fp}: {e}");
                        }
                    }
                    CapabilityId::ACTIONS => {
                        if let Err(e) = serve_actions(&handlers, &peer_fp, stream).await {
                            error!("Couldn't answer an action from {peer_fp}: {e}");
                        }
                    }
                    CapabilityId::FIND => {
                        if let Err(e) = serve_ring(&handlers, &peer_fp, stream).await {
                            error!("Couldn't answer a ring from {peer_fp}: {e}");
                        }
                    }
                    CapabilityId::MEDIA => {
                        if let Err(e) = serve_media(&handlers, &peer_fp, stream).await {
                            error!("Couldn't answer a media request from {peer_fp}: {e}");
                        }
                    }
                    CapabilityId::CALLS => {
                        if let Err(e) = serve_calls(&handlers, &peer_fp, stream).await {
                            error!("Couldn't answer a calls request from {peer_fp}: {e}");
                        }
                    }
                    CapabilityId::FILES => {
                        if let Err(e) = serve_files(&mux, &handlers, &peer_fp, stream).await {
                            error!("Couldn't answer a files request from {peer_fp}: {e}");
                        }
                    }
                    CapabilityId::MESSAGES => {
                        if let Err(e) = serve_messages(&handlers, &peer_fp, stream).await {
                            error!("Couldn't answer a messages request from {peer_fp}: {e}");
                        }
                    }
                    unknown => {
                        debug!("Received unsupported capability stream {unknown:?} from {peer_fp}");
                    }
                }
            });
        }
    });
}

impl SessionMultiplexer {
    pub(crate) async fn ask<M: prost::Message, R: prost::Message + Default>(
        &self,
        capability: CapabilityId,
        message: &M,
        max_bytes: usize,
    ) -> Result<R, TransportError> {
        let (mut send, mut recv) = self.open_stream(capability).await?;
        write_msg(&mut send, message, max_bytes).await?;
        let _ = send.finish();
        read_msg(&mut recv, max_bytes).await
    }

    /// `on_progress` receives (bytes sent, file size) after each chunk is written.
    pub async fn send_file_to_peer<F>(
        &self,
        file_path: &Path,
        transfer_id: String,
        on_progress: Option<F>,
    ) -> Result<u64, transfer::TransferError>
    where
        F: Fn(u64, u64),
    {
        let (mut send, mut recv) = self
            .open_stream(CapabilityId::FILE_TRANSFER)
            .await
            .map_err(transfer::TransferError::Transport)?;

        send_file(&mut send, &mut recv, file_path, transfer_id, on_progress).await
    }

    pub async fn send_clipboard_to_peer(
        &self,
        format: ClipboardFormat,
        payload: Vec<u8>,
        query: &CapabilityQuery,
    ) -> Result<ClipboardAck, clipboard::ClipboardError> {
        let (mut send, mut recv) = self
            .open_stream(CapabilityId::CLIPBOARD)
            .await
            .map_err(clipboard::ClipboardError::Transport)?;

        self.clipboard()
            .send_update(&mut send, &mut recv, format, payload, query)
            .await
    }

    pub async fn send_notification_to_peer(
        &self,
        body: NotificationBody,
        query: &CapabilityQuery,
    ) -> Result<(), notifications::NotificationError> {
        let (mut send, mut recv) = self
            .open_stream(CapabilityId::NOTIFICATIONS)
            .await
            .map_err(notifications::NotificationError::Transport)?;

        notifications::send(&mut send, &mut recv, body, query).await
    }
}
