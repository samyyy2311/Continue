// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Notify};
use tracing::{debug, error, info, warn};

use capabilities::CapabilityQuery;
use clipboard::{ClipboardAck, ClipboardFormat, ClipboardUpdate};
use notifications::{
    NotificationAck, NotificationActionInvoke, NotificationDismiss, NotificationDispatcher,
    NotificationPost,
};
use prost::Message;
use protocol::CapabilityId;
use transfer::{receive_file, send_file, ReceivedFile};

use crate::device::{PeerDevice, ThisDevice};
use crate::incoming::{IncomingFiles, SaveFolder};
use crate::multiplexer::{OnPeerUpdate, PeerUpdate, SessionMultiplexer};

/// How long a question waits for the user before it counts as declined.
pub const PROMPT_TIMEOUT: Duration = Duration::from_secs(30);

/// Something a device set to Ask is trying to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionRequest {
    pub peer: String,
    pub capability: CapabilityId,
    /// The file name, for files.
    pub detail: Option<String>,
    /// When the core stops waiting and declines. Apps should drop the question then too.
    pub deadline: tokio::time::Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Just this once.
    Allow,
    /// This one, and everything of this kind from the device from now on.
    AlwaysAllow,
    Decline,
}

/// Shows a request to the user and hands back a receiver for the answer. Dropping the sender
/// counts as declining.
pub type PermissionPrompt =
    Arc<dyn Fn(PermissionRequest) -> oneshot::Receiver<PermissionDecision> + Send + Sync>;

/// Called with the sender's fingerprint and what it sent.
pub type OnReceived<T> = Arc<dyn Fn(&str, T) + Send + Sync>;

/// Callbacks and configuration for active capabilities over a multiplexed session.
#[derive(Clone)]
pub struct SessionCapabilityHandlers {
    pub save_folder: SaveFolder,
    pub incoming: IncomingFiles,
    pub notification_dispatcher: Arc<NotificationDispatcher>,
    pub on_file_received: Option<OnReceived<ReceivedFile>>,
    pub on_clipboard_received: Option<OnReceived<ClipboardUpdate>>,
    pub on_notification_received: Option<OnReceived<NotificationPost>>,
    pub on_notification_action: Option<OnReceived<NotificationActionInvoke>>,
    pub on_notification_dismiss: Option<OnReceived<NotificationDismiss>>,
    /// How this device introduces itself to peers.
    pub this_device: ThisDevice,
    /// Called with what a peer says about itself each time a session starts.
    pub on_device_info: Option<OnReceived<PeerDevice>>,
    /// Called with the peer's battery when a session starts and each time it changes.
    pub on_device_status: Option<OnReceived<protocol::v1::DeviceStatus>>,
    pub permission_store: Option<Arc<permissions::PermissionStore>>,
    pub permission_prompt: Option<PermissionPrompt>,
    /// Keeps to one question at a time, so a batch of files asks once when the first answer is
    /// "always".
    prompt_turn: Arc<tokio::sync::Mutex<()>>,
}

impl SessionCapabilityHandlers {
    pub fn new(save_folder: impl Into<PathBuf>) -> Self {
        Self {
            save_folder: SaveFolder::new(save_folder),
            incoming: IncomingFiles::default(),
            notification_dispatcher: Arc::new(NotificationDispatcher::new()),
            on_file_received: None,
            on_clipboard_received: None,
            on_notification_received: None,
            on_notification_action: None,
            on_notification_dismiss: None,
            this_device: ThisDevice::default(),
            on_device_info: None,
            on_device_status: None,
            permission_store: None,
            permission_prompt: None,
            prompt_turn: Arc::default(),
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

    pub fn with_permission_prompt(mut self, prompt: PermissionPrompt) -> Self {
        self.permission_prompt = Some(prompt);
        self
    }

    pub fn with_this_device(mut self, device: ThisDevice) -> Self {
        self.this_device = device;
        self
    }
}

/// Whether the device may send this. A device set to Ask gets a question; no answer, or no
/// way to ask, means no.
async fn permitted(
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    capability: CapabilityId,
    detail: Option<String>,
) -> bool {
    let Some(store) = &handlers.permission_store else {
        return true;
    };
    if let Some(answer) = stored_answer(store, peer, capability) {
        return answer;
    }
    let Some(prompt) = &handlers.permission_prompt else {
        return false;
    };

    // One deadline from arrival covers waiting for a turn and the question itself, so a
    // backlog from one device can't hold up everyone else for longer than that.
    let deadline = tokio::time::Instant::now() + PROMPT_TIMEOUT;
    let Ok(_turn) = tokio::time::timeout_at(deadline, handlers.prompt_turn.lock()).await else {
        return false;
    };
    // An "always" given while this request waited its turn already answers it.
    if let Some(answer) = stored_answer(store, peer, capability) {
        return answer;
    }
    let request = PermissionRequest {
        peer: peer.to_string(),
        capability,
        detail,
        deadline,
    };
    match tokio::time::timeout_at(deadline, prompt(request)).await {
        Ok(Ok(PermissionDecision::Allow)) => true,
        Ok(Ok(PermissionDecision::AlwaysAllow)) => {
            if let Err(error) =
                store.set_persisted_grant(peer, capability, 1, permissions::PersistedGrant::Allow)
            {
                warn!("Could not save the permission for {peer}: {error}");
            }
            true
        }
        _ => false,
    }
}

/// The saved answer, or None when the device is set to Ask.
fn stored_answer(
    store: &permissions::PermissionStore,
    peer: &str,
    capability: CapabilityId,
) -> Option<bool> {
    match store.query_state(peer, capability) {
        Ok(permissions::PermissionState::Allow | permissions::PermissionState::AllowOnce) => {
            Some(true)
        }
        Ok(permissions::PermissionState::Ask) => None,
        Ok(permissions::PermissionState::Deny) | Err(_) => Some(false),
    }
}

/// Spawns the capability router loop processing incoming streams dispatched by the multiplexer.
pub fn spawn_capabilities_dispatcher(
    mux: Arc<SessionMultiplexer>,
    handlers: SessionCapabilityHandlers,
    buffer_size: usize,
) {
    let peer_fingerprint = mux.peer_fingerprint().to_string();
    let on_update: OnPeerUpdate = {
        let (peer, on_info, on_status) = (
            peer_fingerprint.clone(),
            handlers.on_device_info.clone(),
            handlers.on_device_status.clone(),
        );
        Arc::new(move |update| match update {
            PeerUpdate::Info(info) => {
                if let (Some(on_info), Some(device)) = (&on_info, PeerDevice::from_info(&info)) {
                    on_info(&peer, device);
                }
            }
            PeerUpdate::Status(status) if status.battery_percent <= 100 => {
                if let Some(on_status) = &on_status {
                    on_status(&peer, status);
                }
            }
            PeerUpdate::Status(_) => {}
        })
    };
    let mut stream_rx = mux.spawn_router_with(buffer_size, Some(on_update));
    let clipboard = mux.clipboard().clone();

    tokio::spawn(async move {
        while let Some(mut stream) = stream_rx.recv().await {
            let handlers = handlers.clone();
            let peer_fp = peer_fingerprint.clone();
            let clipboard = clipboard.clone();

            tokio::spawn(async move {
                match stream.capability {
                    CapabilityId::FILE_TRANSFER => {
                        debug!("Handling incoming file transfer stream from {peer_fp}");
                        let (handlers_ref, peer_ref) = (&handlers, &peer_fp);
                        let check = |req: &protocol::v1::FileTransferRequest| {
                            let detail = Some(req.file_name.clone());
                            async move {
                                permitted(
                                    handlers_ref,
                                    peer_ref,
                                    CapabilityId::FILE_TRANSFER,
                                    detail,
                                )
                                .await
                            }
                        };

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
                                if let Some(store) = &handlers.permission_store {
                                    store.consume_if_allow_once(
                                        &peer_fp,
                                        CapabilityId::FILE_TRANSFER,
                                    );
                                }
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
                        let is_permitted =
                            permitted(&handlers, &peer_fp, CapabilityId::CLIPBOARD, None).await;

                        let query =
                            CapabilityQuery::negotiated(CapabilityId::CLIPBOARD, is_permitted);

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
                                if let Some(store) = &handlers.permission_store {
                                    store.consume_if_allow_once(&peer_fp, CapabilityId::CLIPBOARD);
                                }
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
                        debug!("Handling incoming notification stream from {peer_fp}");
                        let is_permitted =
                            permitted(&handlers, &peer_fp, CapabilityId::NOTIFICATIONS, None).await;

                        let query =
                            CapabilityQuery::negotiated(CapabilityId::NOTIFICATIONS, is_permitted);

                        if let Err(e) = capabilities::evaluate_capability(&query) {
                            error!("Notification capability not permitted for {peer_fp}: {e}");
                            return;
                        }

                        let raw_bytes = match transport::read_raw_msg(
                            &mut stream.recv_stream,
                            limits::MAX_FRAME_NOTIFICATION_BYTES,
                        )
                        .await
                        {
                            Ok(bytes) => bytes,
                            Err(e) => {
                                error!("Failed to read notification frame from {peer_fp}: {e}");
                                return;
                            }
                        };

                        if handlers.on_notification_action.is_some() {
                            if let Ok(action) = NotificationActionInvoke::decode(raw_bytes.clone()) {
                                if !action.action_id.is_empty()
                                    || !action.reply_text.is_empty()
                                    || (handlers.on_notification_received.is_none()
                                        && handlers.on_notification_dismiss.is_none())
                                {
                                    if let Some(store) = &handlers.permission_store {
                                        store.consume_if_allow_once(
                                            &peer_fp,
                                            CapabilityId::NOTIFICATIONS,
                                        );
                                    }
                                    let notif_id = action.notification_id.clone();
                                    if let Some(cb) = &handlers.on_notification_action {
                                        cb(&peer_fp, action);
                                    }
                                    let ack = NotificationAck {
                                        notification_id: notif_id,
                                        handled: true,
                                    };
                                    let _ = transport::write_msg(
                                        &mut stream.send_stream,
                                        &ack,
                                        limits::MAX_FRAME_NOTIFICATION_BYTES,
                                    )
                                    .await;
                                    return;
                                }
                            }
                        }

                        if let Ok(post) = NotificationPost::decode(raw_bytes.clone()) {
                            let is_post = !post.title.is_empty()
                                || !post.body.is_empty()
                                || post.timestamp > 0
                                || !post.actions.is_empty()
                                || (handlers.on_notification_received.is_some()
                                    && handlers.on_notification_dismiss.is_none());

                            if is_post {
                                if post.body.len() > limits::MAX_NOTIFICATION_BODY_BYTES {
                                    let _ = transport::write_msg(
                                        &mut stream.send_stream,
                                        &NotificationAck {
                                            notification_id: post.notification_id.clone(),
                                            handled: false,
                                        },
                                        limits::MAX_FRAME_NOTIFICATION_BYTES,
                                    )
                                    .await;
                                    error!("Notification body too large from {peer_fp}");
                                    return;
                                }

                                if let Some(store) = &handlers.permission_store {
                                    store.consume_if_allow_once(
                                        &peer_fp,
                                        CapabilityId::NOTIFICATIONS,
                                    );
                                }
                                let notif_id = post.notification_id.clone();
                                if let Some(cb) = &handlers.on_notification_received {
                                    cb(&peer_fp, post);
                                }
                                let ack = NotificationAck {
                                    notification_id: notif_id,
                                    handled: true,
                                };
                                let _ = transport::write_msg(
                                    &mut stream.send_stream,
                                    &ack,
                                    limits::MAX_FRAME_NOTIFICATION_BYTES,
                                )
                                .await;
                                return;
                            }
                        }

                        if let Ok(dismiss) = NotificationDismiss::decode(raw_bytes) {
                            if let Some(store) = &handlers.permission_store {
                                store.consume_if_allow_once(
                                    &peer_fp,
                                    CapabilityId::NOTIFICATIONS,
                                );
                            }
                            let notif_id = dismiss.notification_id.clone();
                            if let Some(cb) = &handlers.on_notification_dismiss {
                                cb(&peer_fp, dismiss);
                            }
                            let ack = NotificationAck {
                                notification_id: notif_id,
                                handled: true,
                            };
                            let _ = transport::write_msg(
                                &mut stream.send_stream,
                                &ack,
                                limits::MAX_FRAME_NOTIFICATION_BYTES,
                            )
                            .await;
                            return;
                        }

                        error!("Unrecognized notification message from {peer_fp}");
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
        dispatcher: &NotificationDispatcher,
        post: NotificationPost,
        query: &CapabilityQuery,
    ) -> Result<NotificationAck, notifications::NotificationError> {
        let (mut send, mut recv) = self
            .open_stream(CapabilityId::NOTIFICATIONS)
            .await
            .map_err(notifications::NotificationError::Transport)?;

        dispatcher
            .send_post(&mut send, &mut recv, post, query)
            .await
    }

    pub async fn send_notification_action_to_peer(
        &self,
        dispatcher: &NotificationDispatcher,
        action: NotificationActionInvoke,
        query: &CapabilityQuery,
    ) -> Result<NotificationAck, notifications::NotificationError> {
        let (mut send, mut recv) = self
            .open_stream(CapabilityId::NOTIFICATIONS)
            .await
            .map_err(notifications::NotificationError::Transport)?;

        dispatcher
            .send_action(&mut send, &mut recv, action, query)
            .await
    }

    pub async fn send_notification_dismiss_to_peer(
        &self,
        dispatcher: &NotificationDispatcher,
        dismiss: NotificationDismiss,
        query: &CapabilityQuery,
    ) -> Result<NotificationAck, notifications::NotificationError> {
        let (mut send, mut recv) = self
            .open_stream(CapabilityId::NOTIFICATIONS)
            .await
            .map_err(notifications::NotificationError::Transport)?;

        dispatcher
            .send_dismiss(&mut send, &mut recv, dismiss, query)
            .await
    }
}
