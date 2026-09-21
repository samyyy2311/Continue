// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod capability;
pub mod error;
pub mod frame;
pub mod version;

pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/continue.v1.rs"));
}

pub use capability::CapabilityId;
pub use error::{FrameError, ProtocolError};
pub use frame::{
    decode_frame, decode_frame_from_buf, decode_raw_frame_from_buf, encode_frame, encode_raw_frame,
    FRAME_HEADER_LEN,
};
pub use version::{
    is_version_supported, negotiate_protocol_version, CURRENT_PROTOCOL_VERSION,
    MAX_SUPPORTED_PROTOCOL_VERSION, MIN_SUPPORTED_PROTOCOL_VERSION,
};
