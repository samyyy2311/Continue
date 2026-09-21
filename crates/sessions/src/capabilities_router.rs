// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, error, info};

use capabilities::{CapabilityQuery, PlatformCapability};
use clipboard::{ClipboardAck, ClipboardFormat, ClipboardSynchronizer, ClipboardUpdate};
use notifications::{NotificationAck, NotificationDispatcher, NotificationPost};
use protocol::CapabilityId;
use transfer::{receive_file, send_file, ReceivedFile};

use crate::multiplexer::SessionMultiplexer;

/// Callbacks and configuration for active capabilities over a multiplexed session.
#[derive(Clone)]
pub struct SessionCapabilityHandlers {
    pub download_dir: PathBuf,
    pub clipboard_sync: Arc<ClipboardSynchronizer>,
    pub notification_dispatcher: Arc<NotificationDispatcher>,
    pub on_file_received: Option<Arc<dyn Fn(ReceivedFile) + Send + Sync>>,
    pub on_clipboard_received: Option<Arc<dyn Fn(ClipboardUpdate) + Send + Sync>>,
    pub on_notification_received: Option<Arc<dyn Fn(NotificationPost) + Send + Sync>>,
}

impl SessionCapabilityHandlers {
    pub fn new(download_dir: impl Into<PathBuf>) -> Self {
        Self {
            download_dir: download_dir.into(),
            clipboard_sync: Arc::new(ClipboardSynchronizer::new()),
            notification_dispatcher: Arc::new(NotificationDispatcher::new()),
            on_file_received: None,
            on_clipboard_received: None,
            on_notification_received: None,
        }
    }
}

/// Spawns the capability router loop processing incoming streams dispatched by the multiplexer.
pub fn spawn_capabilities_dispatcher(
    mux: Arc<SessionMultiplexer>,
    handlers: SessionCapabilityHandlers,
    buffer_size: usize,
) {
    let mut stream_rx = mux.spawn_router(buffer_size);
    let peer_fingerprint = mux.peer_fingerprint().to_string();

    tokio::spawn(async move {
        while let Some(mut stream) = stream_rx.recv().await {
            let handlers = handlers.clone();
            let peer_fp = peer_fingerprint.clone();

            tokio::spawn(async move {
                match stream.capability {
                    CapabilityId::FILE_TRANSFER => {
                        debug!("Handling incoming file transfer stream from {peer_fp}");
                        match receive_file(
                            &mut stream.send_stream,
                            &mut stream.recv_stream,
                            &handlers.download_dir,
                            None::<fn(&protocol::v1::FileTransferRequest) -> bool>,
                            None::<fn(u64, u64)>,
                        )
                        .await
                        {
                            Ok(received) => {
                                info!(
                                    "Successfully received file {} ({} bytes) from {peer_fp}",
                                    received.file_name, received.bytes_received
                                );
                                if let Some(cb) = &handlers.on_file_received {
                                    cb(received);
                                }
                            }
                            Err(e) => {
                                error!("Failed to receive file from {peer_fp}: {e}");
                            }
                        }
                    }
                    CapabilityId::CLIPBOARD => {
                        debug!("Handling incoming clipboard stream from {peer_fp}");
                        let query = CapabilityQuery {
                            capability: PlatformCapability::ClipboardRead,
                            platform_available: true,
                            app_permitted: true,
                            peer_trusted: true,
                            session_negotiated: true,
                        };

                        let on_received = handlers.on_clipboard_received.clone();
                        let result = handlers
                            .clipboard_sync
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
                                    cb(update);
                                }
                            }
                            Err(e) => {
                                error!("Failed to process clipboard update from {peer_fp}: {e}");
                            }
                        }
                    }
                    CapabilityId::NOTIFICATIONS => {
                        debug!("Handling incoming notification stream from {peer_fp}");
                        let query = CapabilityQuery {
                            capability: PlatformCapability::NotificationReceive,
                            platform_available: true,
                            app_permitted: true,
                            peer_trusted: true,
                            session_negotiated: true,
                        };

                        let on_received = handlers.on_notification_received.clone();
                        let result = handlers
                            .notification_dispatcher
                            .receive_post(
                                &mut stream.send_stream,
                                &mut stream.recv_stream,
                                &query,
                                |_post| Ok(()),
                            )
                            .await;

                        match result {
                            Ok(post) => {
                                if let Some(cb) = on_received {
                                    cb(post);
                                }
                            }
                            Err(e) => {
                                error!("Failed to process notification post from {peer_fp}: {e}");
                            }
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
    pub async fn send_file_to_peer(
        &self,
        file_path: &Path,
        transfer_id: String,
    ) -> Result<u64, transfer::TransferError> {
        let (mut send, mut recv) = self
            .open_stream(CapabilityId::FILE_TRANSFER)
            .await
            .map_err(transfer::TransferError::Transport)?;

        send_file(
            &mut send,
            &mut recv,
            file_path,
            transfer_id,
            None::<fn(u64, u64)>,
        )
        .await
    }

    pub async fn send_clipboard_to_peer(
        &self,
        synchronizer: &ClipboardSynchronizer,
        format: ClipboardFormat,
        payload: Vec<u8>,
        query: &CapabilityQuery,
    ) -> Result<ClipboardAck, clipboard::ClipboardError> {
        let (mut send, mut recv) = self
            .open_stream(CapabilityId::CLIPBOARD)
            .await
            .map_err(|e| clipboard::ClipboardError::Transport(e.to_string()))?;

        synchronizer
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
            .map_err(|e| notifications::NotificationError::Transport(e.to_string()))?;

        dispatcher
            .send_post(&mut send, &mut recv, post, query)
            .await
    }
}
