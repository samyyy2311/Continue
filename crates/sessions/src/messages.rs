// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use limits::MAX_FRAME_MESSAGES_BYTES;
use protocol::v1::{
    messages_message, Contact, Conversation, MessagesMessage, MessagesReply, TextMessage,
};
use protocol::CapabilityId;
use transport::{read_msg, write_msg, TransportError};

use crate::capabilities_router::{allowed, off_runtime, SessionCapabilityHandlers};
use crate::multiplexer::{IncomingCapabilityStream, SessionMultiplexer};

/// Most conversations, and most texts of one, a page holds.
pub const MAX_CONVERSATIONS: u32 = 50;
pub const MAX_TEXTS: u32 = 200;
pub const MAX_CONTACTS: u32 = 500;

/// Called off the async runtime.
pub trait MessageStore: Send + Sync {
    /// Newest first. None when the device can't see its messages.
    fn conversations(&self, limit: u32) -> Option<Vec<Conversation>>;
    /// The latest texts of one conversation, oldest first.
    fn conversation(&self, id: &str, limit: u32) -> Option<Vec<TextMessage>>;
    /// Whether the text was handed to the carrier.
    fn send(&self, address: &str, body: &str) -> bool;
    /// Favourites first, then by name. None without access to contacts.
    fn contacts(&self, limit: u32) -> Option<Vec<Contact>>;
}

pub(crate) async fn serve_messages(
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    mut stream: IncomingCapabilityStream,
) -> Result<(), TransportError> {
    let message: MessagesMessage =
        read_msg(&mut stream.recv_stream, MAX_FRAME_MESSAGES_BYTES).await?;
    let mut reply = MessagesReply::default();
    match (message.body, &handlers.message_store) {
        // Says nothing about the texts themselves, so it needs no permission.
        (Some(messages_message::Body::Changed(_)), _) => {
            reply.available = true;
            if let Some(on_changed) = &handlers.on_messages_changed {
                on_changed(peer, ());
            }
        }
        (Some(body), Some(store)) if allowed(handlers, peer, CapabilityId::MESSAGES) => {
            let store = store.clone();
            match body {
                messages_message::Body::Conversations(list) => {
                    let limit = list.limit.min(MAX_CONVERSATIONS);
                    let found = off_runtime(move || store.conversations(limit)).await;
                    reply.available = found.is_some();
                    reply.conversations = found.unwrap_or_default();
                }
                messages_message::Body::Read(read) => {
                    let limit = read.limit.min(MAX_TEXTS);
                    let id = read.conversation_id;
                    let found = off_runtime(move || store.conversation(&id, limit)).await;
                    reply.available = found.is_some();
                    reply.messages = found.unwrap_or_default();
                }
                messages_message::Body::Send(send) => {
                    reply.available = true;
                    reply.sent = off_runtime(move || Some(store.send(&send.address, &send.body)))
                        .await
                        .unwrap_or(false);
                }
                messages_message::Body::Contacts(list) => {
                    let limit = list.limit.min(MAX_CONTACTS);
                    let found = off_runtime(move || store.contacts(limit)).await;
                    reply.available = found.is_some();
                    reply.contacts = found.unwrap_or_default();
                }
                messages_message::Body::Changed(_) => {}
            }
        }
        _ => {}
    }
    write_msg(&mut stream.send_stream, &reply, MAX_FRAME_MESSAGES_BYTES).await
}

impl SessionMultiplexer {
    pub async fn send_messages_message(
        &self,
        body: messages_message::Body,
    ) -> Result<MessagesReply, TransportError> {
        let message = MessagesMessage { body: Some(body) };
        self.ask(CapabilityId::MESSAGES, &message, MAX_FRAME_MESSAGES_BYTES)
            .await
    }
}
