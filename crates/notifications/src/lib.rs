// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod dispatcher;
pub mod error;
pub mod wire;

pub use dispatcher::NotificationDispatcher;
pub use error::NotificationError;
pub use protocol::v1::{
    NotificationAck, NotificationAction, NotificationActionInvoke, NotificationDismiss,
    NotificationPost,
};
