// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod actions;
pub mod auto_connect;
pub mod backoff;
pub mod calls;
pub mod capabilities_router;
pub mod device;
pub mod error;
pub mod files;
pub mod find;
pub mod incoming;
pub mod keepalive;
pub mod listener;
pub mod media;
pub mod messages;
pub mod multiplexer;
pub mod photos;
pub mod pointer;
pub mod registry;
pub mod search;
pub mod session;
pub mod snippets;
pub mod state;
pub mod video;

pub use actions::ComputerActions;
pub use auto_connect::connect_paired_peers;
pub use backoff::ReconnectPolicy;
pub use calls::CallControl;
pub use capabilities_router::{
    spawn_capabilities_dispatcher, OnReceived, SessionCapabilityHandlers,
};
pub use device::{clean_name, this_platform, PeerDevice, ThisDevice};
pub use error::SessionError;
pub use find::Ringer;
pub use incoming::{IncomingEvent, IncomingFile, IncomingFiles, IncomingListener, SaveFolder};
pub use keepalive::KeepaliveTracker;
pub use listener::{accept_peers, listen_for_peers, remember_peer_address, DEFAULT_LISTEN_PORT};
pub use media::MediaControl;
pub use messages::{MessageStore, MAX_CONTACTS, MAX_CONVERSATIONS, MAX_TEXTS};
pub use multiplexer::{
    close_code, open_capability_stream, read_capability_stream_header, IncomingCapabilityStream,
    SessionMultiplexer,
};
pub use photos::{PhotoLibrary, MAX_PHOTOS};
pub use pointer::{PointerControl, PointerLeave, PointerLeaves, PointerTarget};
pub use registry::{Direction, RegistryConfig, SessionRegistry, StateListener, RESEND_WAIT};
pub use search::PhoneSearch;
pub use session::Session;
pub use snippets::SnippetStore;
pub use state::SessionState;
pub use video::{
    CameraSource, ScreenSource, VideoControl, VideoFeed, VideoFrames, VideoSource, Watched,
};
