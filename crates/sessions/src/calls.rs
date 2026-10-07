// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use limits::MAX_FRAME_CALLS_BYTES;
use protocol::v1::{call_action, calls_message, CallsMessage, CallsReply};
use protocol::CapabilityId;
use transport::{read_msg, write_msg, TransportError};

use crate::capabilities_router::{allowed, off_runtime, SessionCapabilityHandlers};
use crate::multiplexer::{IncomingCapabilityStream, SessionMultiplexer};

/// Called off the async runtime.
pub trait CallControl: Send + Sync {
    /// Whether there was a ringing call to answer.
    fn answer(&self) -> bool;
    /// Whether there was a call to decline or hang up.
    fn decline(&self) -> bool;
    /// Whether the ringtone was silenced.
    fn silence(&self) -> bool;
    /// Whether the call to `number` was started.
    fn dial(&self, number: &str) -> bool;
}

pub(crate) async fn serve_calls(
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    mut stream: IncomingCapabilityStream,
) -> Result<(), TransportError> {
    let message: CallsMessage = read_msg(&mut stream.recv_stream, MAX_FRAME_CALLS_BYTES).await?;
    let mut reply = CallsReply::default();
    match message.body {
        Some(calls_message::Body::Call(call)) => {
            reply.done = allowed(handlers, peer, CapabilityId::CALLS);
            if let (true, Some(on_call)) = (reply.done, &handlers.on_call) {
                on_call(peer, call);
            }
        }
        Some(calls_message::Body::Action(action)) => {
            if let Some(control) = handlers.call_control.clone() {
                if allowed(handlers, peer, CapabilityId::CALLS) {
                    let kind = action.kind();
                    reply.done = off_runtime(move || {
                        Some(match kind {
                            call_action::Kind::Unspecified => false,
                            call_action::Kind::Answer => control.answer(),
                            call_action::Kind::Decline => control.decline(),
                            call_action::Kind::Silence => control.silence(),
                            call_action::Kind::Dial => control.dial(&action.number),
                        })
                    })
                    .await
                    .unwrap_or(false);
                }
            }
        }
        None => {}
    }
    write_msg(&mut stream.send_stream, &reply, MAX_FRAME_CALLS_BYTES).await
}

impl SessionMultiplexer {
    pub async fn send_calls_message(
        &self,
        body: calls_message::Body,
    ) -> Result<CallsReply, TransportError> {
        let message = CallsMessage { body: Some(body) };
        self.ask(CapabilityId::CALLS, &message, MAX_FRAME_CALLS_BYTES)
            .await
    }
}
