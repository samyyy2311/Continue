// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use limits::MAX_FRAME_SEARCH_BYTES;
use protocol::v1::{SearchReply, SearchRequest, SearchResult};
use protocol::CapabilityId;
use transport::{read_msg, write_msg, TransportError};

use crate::capabilities_router::{allowed, off_runtime, SessionCapabilityHandlers};
use crate::multiplexer::{IncomingCapabilityStream, SessionMultiplexer};

/// The most results one search sends back.
pub const MAX_RESULTS: u32 = 30;

/// Called off the async runtime. None when there's nothing this device may look through.
pub trait PhoneSearch: Send + Sync {
    fn search(&self, query: &str, limit: u32) -> Option<Vec<SearchResult>>;
}

pub(crate) async fn serve_search(
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    mut stream: IncomingCapabilityStream,
) -> Result<(), TransportError> {
    let request: SearchRequest = read_msg(&mut stream.recv_stream, MAX_FRAME_SEARCH_BYTES).await?;
    let search = handlers
        .phone_search
        .clone()
        .filter(|_| allowed(handlers, peer, CapabilityId::SEARCH));
    let query = request.query.trim().to_string();
    let results = match search {
        Some(search) if !query.is_empty() => {
            off_runtime(move || search.search(&query, MAX_RESULTS)).await
        }
        _ => None,
    };
    let reply = SearchReply {
        available: results.is_some(),
        results: results
            .unwrap_or_default()
            .into_iter()
            .take(MAX_RESULTS as usize)
            .collect(),
    };
    write_msg(&mut stream.send_stream, &reply, MAX_FRAME_SEARCH_BYTES).await
}

impl SessionMultiplexer {
    pub async fn search_peer(&self, query: String) -> Result<SearchReply, TransportError> {
        self.ask(
            CapabilityId::SEARCH,
            &SearchRequest { query },
            MAX_FRAME_SEARCH_BYTES,
        )
        .await
    }
}
