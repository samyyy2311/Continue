// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("Frame error: {0}")]
    Frame(#[from] protocol::FrameError),

    #[error("Transport error: {0}")]
    Transport(#[from] transport::TransportError),

    #[error("Capability error: {0}")]
    Capability(#[from] capabilities::CapabilityError),

    #[error("Notification body too large: {size} bytes (limit: {limit})")]
    BodyTooLarge { size: usize, limit: usize },

    #[error("Notification not found: {0}")]
    NotFound(String),

    #[error("Action not handled: {0}")]
    ActionFailed(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
