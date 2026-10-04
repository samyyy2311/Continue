// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Streaming file transfer, one QUIC stream per file.

pub mod error;
mod partial;
pub mod receiver;
pub mod sanitizer;
pub mod sender;

pub use error::TransferError;
pub use receiver::{receive_file, ReceivedFile};
pub use sanitizer::sanitize_filename;
pub use sender::{compute_file_sha256, send_file};
