// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod auto_connect;
pub mod backoff;
pub mod capabilities_router;
pub mod error;
pub mod keepalive;
pub mod listener;
pub mod multiplexer;
pub mod registry;
pub mod session;
pub mod state;

pub use auto_connect::connect_paired_peers;
pub use backoff::ReconnectPolicy;
pub use capabilities_router::{
    spawn_capabilities_dispatcher, PermissionDecision, PermissionPrompt, PermissionRequest,
    SessionCapabilityHandlers, PROMPT_TIMEOUT,
};
pub use error::SessionError;
pub use keepalive::KeepaliveTracker;
pub use listener::{accept_peers, listen_for_peers, remember_peer_address, DEFAULT_LISTEN_PORT};
pub use multiplexer::{
    close_code, open_capability_stream, read_capability_stream_header, IncomingCapabilityStream,
    SessionMultiplexer,
};
pub use registry::{Direction, RegistryConfig, SessionRegistry, StateListener};
pub use session::Session;
pub use state::SessionState;
