// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use limits::MAX_FRAME_RING_BYTES;
use protocol::v1::{Ring, RingReply};
use protocol::CapabilityId;
use transport::{read_msg, write_msg, TransportError};

use crate::capabilities_router::{allowed, off_runtime, SessionCapabilityHandlers};
use crate::multiplexer::{IncomingCapabilityStream, SessionMultiplexer};

/// Called off the async runtime.
pub trait Ringer: Send + Sync {
    fn ring(&self, on: bool) -> bool;
}

pub(crate) async fn serve_ring(
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    mut stream: IncomingCapabilityStream,
) -> Result<(), TransportError> {
    let ring: Ring = read_msg(&mut stream.recv_stream, MAX_FRAME_RING_BYTES).await?;
    let mut reply = RingReply::default();
    if let Some(ringer) = handlers.ringer.clone() {
        if allowed(handlers, peer, CapabilityId::FIND) {
            reply.done = off_runtime(move || Some(ringer.ring(ring.on)))
                .await
                .unwrap_or(false);
        }
    }
    write_msg(&mut stream.send_stream, &reply, MAX_FRAME_RING_BYTES).await
}

impl SessionMultiplexer {
    pub async fn ring_peer(&self, on: bool) -> Result<RingReply, TransportError> {
        self.ask(CapabilityId::FIND, &Ring { on }, MAX_FRAME_RING_BYTES)
            .await
    }
}
