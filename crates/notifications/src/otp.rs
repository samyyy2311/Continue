// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use protocol::v1::{NotificationAction, NotificationPost};

pub const ACTION_COPY_OTP: &str = "copy_otp";

const OTP_KEYWORDS: &[&str] = &[
    "code",
    "otp",
    "verification",
    "passcode",
    "security code",
    "auth code",
    "pin",
];

pub fn extract_otp(title: &str, body: &str) -> Option<String> {
    let combined = format!("{} {}", title, body).to_ascii_lowercase();

    let has_keyword = OTP_KEYWORDS.iter().any(|&kw| combined.contains(kw));
    if !has_keyword {
        return None;
    }

    for token in body.split(|c: char| {
        c.is_whitespace() || c == ':' || c == '.' || c == ',' || c == '#' || c == '-'
    }) {
        let trimmed = token.trim_matches(|c: char| !c.is_ascii_digit());
        if trimmed.len() >= 4 && trimmed.len() <= 8 && trimmed.chars().all(|c| c.is_ascii_digit()) {
            return Some(trimmed.to_string());
        }
    }

    None
}

pub fn enrich_with_otp_action(mut post: NotificationPost) -> NotificationPost {
    if let Some(code) = extract_otp(&post.title, &post.body) {
        if !post.actions.iter().any(|a| a.action_id == ACTION_COPY_OTP) {
            post.actions.insert(
                0,
                NotificationAction {
                    action_id: ACTION_COPY_OTP.to_string(),
                    label: format!("Copy {}", code),
                    is_reply: false,
                },
            );
        }
    }
    post
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_verification_code() {
        let code = extract_otp("Google", "G-839201 is your verification code.");
        assert_eq!(code, Some("839201".to_string()));

        let code2 = extract_otp(
            "Bank Alert",
            "Your one-time OTP is 582194 for account login.",
        );
        assert_eq!(code2, Some("582194".to_string()));
    }

    #[test]
    fn ignores_non_otp_notifications() {
        let code = extract_otp("WhatsApp", "Hey, are you free for a call tonight?");
        assert_eq!(code, None);
    }

    #[test]
    fn enriches_notification_with_copy_action() {
        let post = NotificationPost {
            notification_id: "notif-1".to_string(),
            package_name: "com.example.bank".to_string(),
            app_name: "Bank".to_string(),
            title: "Security Verification".to_string(),
            body: "Your login passcode is 948123.".to_string(),
            timestamp: 1000,
            actions: vec![NotificationAction {
                action_id: "dismiss".to_string(),
                label: "Dismiss".to_string(),
                is_reply: false,
            }],
        };

        let enriched = enrich_with_otp_action(post);
        assert_eq!(enriched.actions.len(), 2);
        assert_eq!(enriched.actions[0].action_id, ACTION_COPY_OTP);
        assert_eq!(enriched.actions[0].label, "Copy 948123");
    }
}
