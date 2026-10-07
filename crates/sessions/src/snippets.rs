// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use limits::MAX_FRAME_SNIPPETS_BYTES;
use protocol::v1::{Snippet, Snippets, SnippetsReply};
use protocol::CapabilityId;
use transport::{read_msg, write_msg, TransportError};

use crate::capabilities_router::{allowed, off_runtime, SessionCapabilityHandlers};
use crate::multiplexer::{IncomingCapabilityStream, SessionMultiplexer};

/// Called off the async runtime with every snippet a peer has.
pub trait SnippetStore: Send + Sync {
    fn merge(&self, peer: &str, snippets: Vec<Snippet>);
}

pub(crate) async fn serve_snippets(
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    mut stream: IncomingCapabilityStream,
) -> Result<(), TransportError> {
    let sent: Snippets = read_msg(&mut stream.recv_stream, MAX_FRAME_SNIPPETS_BYTES).await?;
    let store = handlers
        .snippet_store
        .clone()
        .filter(|_| allowed(handlers, peer, CapabilityId::SNIPPETS));
    let taken = store.is_some();
    if let Some(store) = store {
        let peer = peer.to_string();
        off_runtime(move || {
            store.merge(&peer, sent.snippets);
            Some(())
        })
        .await;
    }
    write_msg(
        &mut stream.send_stream,
        &SnippetsReply { taken },
        MAX_FRAME_SNIPPETS_BYTES,
    )
    .await
}

impl SessionMultiplexer {
    pub async fn send_snippets(&self, snippets: Vec<Snippet>) -> Result<bool, TransportError> {
        let reply: SnippetsReply = self
            .ask(
                CapabilityId::SNIPPETS,
                &Snippets { snippets },
                MAX_FRAME_SNIPPETS_BYTES,
            )
            .await?;
        Ok(reply.taken)
    }
}
