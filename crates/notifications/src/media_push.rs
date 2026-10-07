// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use protocol::v1::{NotificationAction, NotificationPost};

pub const MEDIA_PUSH_PACKAGE: &str = "continue.media.push";
pub const ACTION_COPY_MEDIA_CLIPBOARD: &str = "copy_media_clipboard";
pub const ACTION_SAVE_MEDIA_DOWNLOADS: &str = "save_media_downloads";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaOrigin {
    CameraPhoto,
    Screenshot,
    SavedImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaPushItem {
    pub media_id: String,
    pub file_name: String,
    pub mime_type: String,
    pub file_size_bytes: u64,
    pub origin: MediaOrigin,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaPushAction {
    CopyToClipboard,
    SaveToDownloads,
}

impl MediaPushItem {
    pub fn to_notification_post(&self) -> NotificationPost {
        let (title, default_prefix) = match self.origin {
            MediaOrigin::Screenshot => ("New screenshot captured", "Screenshot"),
            MediaOrigin::CameraPhoto => ("New photo captured", "Photo"),
            MediaOrigin::SavedImage => ("New image saved", "Image"),
        };

        let size_str = format_byte_size(self.file_size_bytes);
        let body = if self.file_name.is_empty() {
            format!("{default_prefix} ({size_str})")
        } else {
            format!("{} ({size_str})", self.file_name)
        };

        NotificationPost {
            notification_id: format!("media_{}", self.media_id),
            package_name: MEDIA_PUSH_PACKAGE.to_string(),
            app_name: "Instant Media".to_string(),
            title: title.to_string(),
            body,
            timestamp: self.timestamp_ms,
            actions: vec![
                NotificationAction {
                    action_id: format!("{}:{}", ACTION_COPY_MEDIA_CLIPBOARD, self.media_id),
                    label: "Copy to Clipboard".to_string(),
                    is_reply: false,
                },
                NotificationAction {
                    action_id: format!("{}:{}", ACTION_SAVE_MEDIA_DOWNLOADS, self.media_id),
                    label: "Save to Downloads".to_string(),
                    is_reply: false,
                },
            ],
        }
    }
}

pub fn parse_media_push_action(action_id: &str) -> Option<(MediaPushAction, &str)> {
    if let Some(media_id) = action_id.strip_prefix(&format!("{}:", ACTION_COPY_MEDIA_CLIPBOARD)) {
        Some((MediaPushAction::CopyToClipboard, media_id))
    } else if let Some(media_id) = action_id.strip_prefix(&format!("{}:", ACTION_SAVE_MEDIA_DOWNLOADS)) {
        Some((MediaPushAction::SaveToDownloads, media_id))
    } else {
        None
    }
}

fn format_byte_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{} KB", bytes / KB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_push_creates_notification_with_actions() {
        let item = MediaPushItem {
            media_id: "sc-123".into(),
            file_name: "Screenshot_20261008.png".into(),
            mime_type: "image/png".into(),
            file_size_bytes: 2 * 1024 * 1024 + 512 * 1024,
            origin: MediaOrigin::Screenshot,
            timestamp_ms: 1700000000,
        };

        let post = item.to_notification_post();
        assert_eq!(post.notification_id, "media_sc-123");
        assert_eq!(post.package_name, MEDIA_PUSH_PACKAGE);
        assert_eq!(post.title, "New screenshot captured");
        assert_eq!(post.body, "Screenshot_20261008.png (2.5 MB)");
        assert_eq!(post.actions.len(), 2);
        assert_eq!(post.actions[0].action_id, "copy_media_clipboard:sc-123");
        assert_eq!(post.actions[1].action_id, "save_media_downloads:sc-123");
    }

    #[test]
    fn parses_media_push_action_payload() {
        let parsed_copy = parse_media_push_action("copy_media_clipboard:sc-123");
        assert_eq!(parsed_copy, Some((MediaPushAction::CopyToClipboard, "sc-123")));

        let parsed_save = parse_media_push_action("save_media_downloads:img-99");
        assert_eq!(parsed_save, Some((MediaPushAction::SaveToDownloads, "img-99")));

        let parsed_unknown = parse_media_push_action("unrelated_action:sc-123");
        assert_eq!(parsed_unknown, None);
    }
}
