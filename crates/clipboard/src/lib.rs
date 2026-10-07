// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod context;
pub mod error;
pub mod guard;
pub mod sync;

pub use context::{classify_context, ContextAction};
pub use error::ClipboardError;
pub use guard::{evaluate_clipboard_formats, SensitiveGuardDecision};
pub use protocol::v1::{ClipboardAck, ClipboardFormat, ClipboardUpdate};
pub use sync::ClipboardSynchronizer;
