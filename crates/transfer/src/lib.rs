// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Streaming file transfer over dedicated QUIC streams (Phase 1B).

pub mod error;
pub mod receiver;
pub mod sanitizer;
pub mod sender;
mod wire;

pub use error::TransferError;
pub use receiver::{receive_file, ReceivedFile};
pub use sanitizer::sanitize_filename;
pub use sender::{compute_file_sha256, send_file};
