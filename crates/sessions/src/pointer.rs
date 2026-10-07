// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! The computer's mouse and keyboard used on the phone: the computer sends input while its
//! pointer is over there, and the phone says when it goes back.

use limits::MAX_FRAME_POINTER_BYTES;
use protocol::v1::{PointerInput, PointerLeft, PointerReply, PointerStart};
use protocol::CapabilityId;
use tokio::sync::mpsc;
use transport::{read_msg, write_msg, TransportError};

use crate::capabilities_router::{allowed, off_runtime, SessionCapabilityHandlers};
use crate::multiplexer::{IncomingCapabilityStream, SessionMultiplexer};

/// Tells the computer the pointer went back over the edge, with where along it.
pub type PointerLeave = Box<dyn Fn(f32) + Send + Sync>;

/// Called off the async runtime, with input in the order it was sent.
pub trait PointerTarget: Send + Sync {
    /// Shows the pointer. False if this device can't take input right now.
    fn start(&self, start: PointerStart, leave: PointerLeave) -> bool;
    fn input(&self, input: PointerInput);
    /// Hides the pointer and lets go of anything still pressed.
    fn stop(&self);
}

pub(crate) async fn serve_pointer(
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    mut stream: IncomingCapabilityStream,
) -> Result<(), TransportError> {
    let start: PointerStart = read_msg(&mut stream.recv_stream, MAX_FRAME_POINTER_BYTES).await?;
    let target = handlers
        .pointer_target
        .clone()
        .filter(|_| allowed(handlers, peer, CapabilityId::POINTER));
    let (leaves, mut left) = mpsc::unbounded_channel();
    let started = match target.clone() {
        Some(target) => off_runtime(move || {
            let leave: PointerLeave = Box::new(move |y| {
                let _ = leaves.send(y);
            });
            Some(target.start(start, leave))
        })
        .await
        .unwrap_or(false),
        None => false,
    };
    let reply = PointerReply { started };
    write_msg(&mut stream.send_stream, &reply, MAX_FRAME_POINTER_BYTES).await?;
    let (Some(target), true) = (target, started) else {
        return Ok(());
    };

    // One worker keeps a press, the moves and the release in order.
    let (inputs, queued) = std::sync::mpsc::channel::<PointerInput>();
    let worker = target.clone();
    let working = tokio::task::spawn_blocking(move || {
        for input in queued {
            worker.input(input);
        }
    });
    // Reading on its own task, so writing a PointerLeft never cancels a read halfway through.
    let mut recv = stream.recv_stream;
    let mut reading = tokio::spawn(async move {
        while let Ok(input) = read_msg::<PointerInput>(&mut recv, MAX_FRAME_POINTER_BYTES).await {
            let _ = inputs.send(input);
        }
    });
    let result = async {
        loop {
            tokio::select! {
                // The computer took its pointer back.
                _ = &mut reading => return Ok(()),
                Some(y) = left.recv() => {
                    write_msg(&mut stream.send_stream, &PointerLeft { y }, MAX_FRAME_POINTER_BYTES)
                        .await?;
                }
            }
        }
    }
    .await;
    reading.abort();
    let _ = working.await;
    let _ = tokio::task::spawn_blocking(move || target.stop()).await;
    let _ = stream.send_stream.finish();
    result
}

/// Dropping it hands the pointer back to the computer.
pub struct PointerControl(quinn::SendStream);

impl PointerControl {
    pub async fn send(&mut self, input: &PointerInput) -> Result<(), TransportError> {
        write_msg(&mut self.0, input, MAX_FRAME_POINTER_BYTES).await
    }
}

impl Drop for PointerControl {
    fn drop(&mut self) {
        let _ = self.0.finish();
    }
}

pub struct PointerLeaves(quinn::RecvStream);

impl PointerLeaves {
    /// Waits for the pointer to come back over the edge; an error once the phone stops.
    pub async fn next(&mut self) -> Result<PointerLeft, TransportError> {
        read_msg(&mut self.0, MAX_FRAME_POINTER_BYTES).await
    }
}

impl SessionMultiplexer {
    /// None if the peer can't take input.
    pub async fn point(
        &self,
        start: PointerStart,
    ) -> Result<Option<(PointerControl, PointerLeaves)>, TransportError> {
        let (mut send, mut recv) = self.open_stream(CapabilityId::POINTER).await?;
        write_msg(&mut send, &start, MAX_FRAME_POINTER_BYTES).await?;
        let reply: PointerReply = read_msg(&mut recv, MAX_FRAME_POINTER_BYTES).await?;
        Ok(reply
            .started
            .then(|| (PointerControl(send), PointerLeaves(recv))))
    }
}
