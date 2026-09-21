// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod error;
pub mod sync;
pub mod wire;

pub use error::ClipboardError;
pub use protocol::v1::{ClipboardAck, ClipboardFormat, ClipboardUpdate};
pub use sync::ClipboardSynchronizer;
