// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use limits::MAX_FRAME_ACTION_BYTES;
use protocol::v1::{computer_action, ComputerAction, ComputerActionReply};
use protocol::CapabilityId;
use transport::{read_msg, write_msg, TransportError};

use crate::capabilities_router::{allowed, off_runtime, SessionCapabilityHandlers};
use crate::multiplexer::{IncomingCapabilityStream, SessionMultiplexer};

/// Called off the async runtime. False if it couldn't be done here.
pub trait ComputerActions: Send + Sync {
    fn act(&self, action: computer_action::Body) -> bool;
}

pub(crate) async fn serve_actions(
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    mut stream: IncomingCapabilityStream,
) -> Result<(), TransportError> {
    let action: ComputerAction = read_msg(&mut stream.recv_stream, MAX_FRAME_ACTION_BYTES).await?;
    let mut reply = ComputerActionReply::default();
    let actions = handlers
        .computer_actions
        .clone()
        .filter(|_| allowed(handlers, peer, CapabilityId::ACTIONS));
    if let (Some(actions), Some(body)) = (actions, action.body) {
        reply.done = off_runtime(move || Some(actions.act(body)))
            .await
            .unwrap_or(false);
    }
    write_msg(&mut stream.send_stream, &reply, MAX_FRAME_ACTION_BYTES).await
}

impl SessionMultiplexer {
    pub async fn act_on_peer(
        &self,
        body: computer_action::Body,
    ) -> Result<ComputerActionReply, TransportError> {
        let action = ComputerAction { body: Some(body) };
        self.ask(CapabilityId::ACTIONS, &action, MAX_FRAME_ACTION_BYTES)
            .await
    }
}
