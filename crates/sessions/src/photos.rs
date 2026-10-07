// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use limits::MAX_FRAME_PHOTOS_BYTES;
use protocol::v1::{photos_message, Photo, PhotosMessage, PhotosReply};
use protocol::CapabilityId;
use transport::{read_msg, write_msg, TransportError};

use crate::capabilities_router::{
    allowed, off_runtime, send_requested_file, SessionCapabilityHandlers,
};
use crate::multiplexer::{IncomingCapabilityStream, SessionMultiplexer};

/// Most photos one list holds, so their thumbnails fit in a frame.
pub const MAX_PHOTOS: u32 = 60;

/// Called off the async runtime.
pub trait PhotoLibrary: Send + Sync {
    /// Newest first, each with a small thumbnail. None when the device can't see its photos.
    fn recent(&self, limit: u32) -> Option<Vec<Photo>>;
    /// Where the photo is, or None when it's gone.
    fn file(&self, id: &str) -> Option<PathBuf>;
}

/// A requested photo follows as a normal file transfer.
pub(crate) async fn serve_photos(
    mux: &SessionMultiplexer,
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    mut stream: IncomingCapabilityStream,
) -> Result<(), TransportError> {
    let message: PhotosMessage = read_msg(&mut stream.recv_stream, MAX_FRAME_PHOTOS_BYTES).await?;
    let mut reply = PhotosReply::default();
    let mut wanted = None;
    match (message.body, &handlers.photo_library) {
        (Some(photos_message::Body::Taken(photo)), _) => {
            reply.available = allowed(handlers, peer, CapabilityId::PHOTOS);
            if let (true, Some(on_taken)) = (reply.available, &handlers.on_photo_taken) {
                on_taken(peer, photo);
            }
        }
        (Some(body), Some(library)) if allowed(handlers, peer, CapabilityId::PHOTOS) => {
            let library = library.clone();
            match body {
                photos_message::Body::List(list) => {
                    let limit = list.limit.min(MAX_PHOTOS);
                    let photos = off_runtime(move || library.recent(limit)).await;
                    reply.available = photos.is_some();
                    reply.photos = photos.unwrap_or_default();
                }
                photos_message::Body::Send(send) => {
                    wanted = off_runtime(move || library.file(&send.id)).await;
                    reply.available = wanted.is_some();
                }
                photos_message::Body::Taken(_) => {}
            }
        }
        _ => {}
    }
    write_msg(&mut stream.send_stream, &reply, MAX_FRAME_PHOTOS_BYTES).await?;
    let _ = stream.send_stream.finish();

    if let Some(path) = wanted {
        send_requested_file(mux, peer, &path).await;
    }
    Ok(())
}

impl SessionMultiplexer {
    pub async fn send_photos_message(
        &self,
        body: photos_message::Body,
    ) -> Result<PhotosReply, TransportError> {
        let message = PhotosMessage { body: Some(body) };
        self.ask(CapabilityId::PHOTOS, &message, MAX_FRAME_PHOTOS_BYTES)
            .await
    }
}
