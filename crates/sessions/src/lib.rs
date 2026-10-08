// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod auto_connect;
pub mod backoff;
pub mod capabilities_router;
pub mod deck;
pub mod device;
pub mod error;
pub mod handoff;
pub mod incoming;
pub mod keepalive;
pub mod listener;
pub mod media_control;
pub mod multiplexer;
pub mod pc_control;
pub mod registry;
pub mod remote_input;
pub mod ring;
pub mod session;
pub mod state;
pub mod telemetry;

pub use auto_connect::connect_paired_peers;
pub use backoff::ReconnectPolicy;
pub use capabilities_router::{
    spawn_capabilities_dispatcher, DeckLayoutHandler, DeckTriggerHandler, MediaCommandHandler,
    OnReceived, PcActionHandler, PermissionDecision, PermissionPrompt, PermissionRequest,
    RemoteInputHandler, RingHandler, SessionCapabilityHandlers, PROMPT_TIMEOUT,
};
pub use deck::DeckDispatcher;
pub use device::{clean_name, this_platform, PeerDevice, ThisDevice};
pub use error::SessionError;
pub use handoff::HandoffDispatcher;
pub use incoming::{IncomingEvent, IncomingFile, IncomingFiles, IncomingListener, SaveFolder};
pub use keepalive::KeepaliveTracker;
pub use listener::{accept_peers, listen_for_peers, remember_peer_address, DEFAULT_LISTEN_PORT};
pub use media_control::MediaControlDispatcher;
pub use multiplexer::{
    close_code, open_capability_stream, read_capability_stream_header, IncomingCapabilityStream,
    SessionMultiplexer,
};
pub use pc_control::PcControlDispatcher;
pub use registry::{Direction, RegistryConfig, SessionRegistry, StateListener, RESEND_WAIT};
pub use remote_input::RemoteInputDispatcher;
pub use ring::{RingDispatcher, DEFAULT_RING_DURATION_SECS};
pub use session::Session;
pub use state::SessionState;
pub use telemetry::{TelemetryAlertEvaluator, TelemetryDispatcher};
