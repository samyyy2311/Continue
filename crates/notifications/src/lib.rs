// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod dispatcher;
pub mod error;
pub mod media_push;
pub mod otp;
pub mod task;

pub use dispatcher::{acknowledge, read, send};
pub use error::NotificationError;
pub use media_push::{
    parse_media_push_action, MediaOrigin, MediaPushAction, MediaPushItem,
    ACTION_COPY_MEDIA_CLIPBOARD, ACTION_SAVE_MEDIA_DOWNLOADS, MEDIA_PUSH_PACKAGE,
};
pub use otp::{enrich_with_otp_action, extract_otp, ACTION_COPY_OTP};
pub use protocol::v1::notification_message::Body;
pub use protocol::v1::{
    NotificationAction, NotificationActionInvoke, NotificationDismiss, NotificationPost,
};
pub use task::TaskCompletionReport;
