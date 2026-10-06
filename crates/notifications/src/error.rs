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

    #[error("Action not handled: {0}")]
    ActionFailed(String),
}
