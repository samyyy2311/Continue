// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod dispatcher;
pub mod error;

pub use dispatcher::{acknowledge, read, send};
pub use error::NotificationError;
pub use protocol::v1::notification_message::Body;
pub use protocol::v1::{
    NotificationAction, NotificationActionInvoke, NotificationDismiss, NotificationMute,
    NotificationPost,
};
