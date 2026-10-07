// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use limits::MAX_FRAME_MEDIA_BYTES;
use protocol::v1::{media_command, media_message, MediaMessage, MediaReply};
use protocol::CapabilityId;
use transport::{read_msg, write_msg, TransportError};

use crate::capabilities_router::{allowed, off_runtime, SessionCapabilityHandlers};
use crate::multiplexer::{IncomingCapabilityStream, SessionMultiplexer};

/// Called off the async runtime.
pub trait MediaControl: Send + Sync {
    /// False when nothing is playing to control.
    fn command(&self, command: media_command::Kind) -> bool;
}

pub(crate) async fn serve_media(
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    mut stream: IncomingCapabilityStream,
) -> Result<(), TransportError> {
    let message: MediaMessage = read_msg(&mut stream.recv_stream, MAX_FRAME_MEDIA_BYTES).await?;
    let mut reply = MediaReply::default();
    match message.body {
        Some(media_message::Body::NowPlaying(playing)) => {
            reply.done = allowed(handlers, peer, CapabilityId::MEDIA);
            if let (true, Some(on_now_playing)) = (reply.done, &handlers.on_now_playing) {
                on_now_playing(peer, playing);
            }
        }
        Some(media_message::Body::Command(command)) => {
            if let Some(control) = handlers.media_control.clone() {
                if allowed(handlers, peer, CapabilityId::MEDIA) {
                    let kind = command.kind();
                    reply.done = off_runtime(move || Some(control.command(kind)))
                        .await
                        .unwrap_or(false);
                }
            }
        }
        None => {}
    }
    write_msg(&mut stream.send_stream, &reply, MAX_FRAME_MEDIA_BYTES).await
}

impl SessionMultiplexer {
    pub async fn send_media_message(
        &self,
        body: media_message::Body,
    ) -> Result<MediaReply, TransportError> {
        let message = MediaMessage { body: Some(body) };
        self.ask(CapabilityId::MEDIA, &message, MAX_FRAME_MEDIA_BYTES)
            .await
    }
}
