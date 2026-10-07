// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextAction {
    OpenUrl(String),
    CopyOtp(String),
    CallPhone(String),
    TrackPackage { carrier: &'static str, tracking_number: String },
}

pub fn classify_context(text: &str) -> Option<ContextAction> {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() > 1024 {
        return None;
    }

    if let Some(url) = detect_url(trimmed) {
        return Some(ContextAction::OpenUrl(url));
    }

    if let Some(tracking) = detect_tracking(trimmed) {
        return Some(tracking);
    }

    if let Some(otp) = detect_otp(trimmed) {
        return Some(ContextAction::CopyOtp(otp));
    }

    if let Some(phone) = detect_phone(trimmed) {
        return Some(ContextAction::CallPhone(phone));
    }

    None
}

fn detect_url(s: &str) -> Option<String> {
    if (s.starts_with("http://") || s.starts_with("https://")) && !s.contains(' ') {
        let host = s.split_once("://")?.1;
        if host.contains('.') && !host.starts_with('.') {
            return Some(s.to_string());
        }
    }
    None
}

fn detect_tracking(s: &str) -> Option<ContextAction> {
    let raw: String = s.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();

    if raw.len() == 18
        && raw[..2].eq_ignore_ascii_case("1Z")
        && raw[2..].chars().all(|c| c.is_ascii_alphanumeric())
    {
        return Some(ContextAction::TrackPackage {
            carrier: "UPS",
            tracking_number: raw.to_ascii_uppercase(),
        });
    }

    if (raw.len() == 12 || raw.len() == 15) && raw.chars().all(|c| c.is_ascii_digit()) {
        return Some(ContextAction::TrackPackage {
            carrier: "FedEx",
            tracking_number: raw,
        });
    }

    if (raw.len() == 20 || raw.len() == 22)
        && (raw.starts_with("94") || raw.starts_with("92") || raw.starts_with("93"))
        && raw.chars().all(|c| c.is_ascii_digit())
    {
        return Some(ContextAction::TrackPackage {
            carrier: "USPS",
            tracking_number: raw,
        });
    }

    None
}

fn detect_otp(s: &str) -> Option<String> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();

    if s.chars().all(|c| c.is_ascii_digit() || c == '-' || c.is_whitespace())
        && digits.len() >= 4
        && digits.len() <= 8
    {
        return Some(digits);
    }

    let lower = s.to_ascii_lowercase();
    let has_keyword = ["code", "otp", "verification", "passcode"]
        .iter()
        .any(|kw| lower.contains(kw));

    if has_keyword {
        for token in s.split(|c: char| c.is_whitespace() || c == ':' || c == '.' || c == ',') {
            let token_digits: String = token.chars().filter(|c| c.is_ascii_digit()).collect();
            if token_digits.len() >= 4 && token_digits.len() <= 8 && token_digits == token.trim_matches(|c: char| !c.is_ascii_digit()) {
                return Some(token_digits);
            }
        }
    }

    None
}

fn detect_phone(s: &str) -> Option<String> {
    let valid_start = s.starts_with('+') || s.starts_with('(') || s.chars().next().is_some_and(|c| c.is_ascii_digit());
    if !valid_start {
        return None;
    }

    let digits = s.chars().filter(|c| c.is_ascii_digit()).count();
    let valid_chars = s.chars().all(|c| c.is_ascii_digit() || c == '+' || c == '-' || c == '(' || c == ')' || c == '.' || c.is_whitespace());

    if valid_chars && (7..=15).contains(&digits) && s.len() <= 24 {
        return Some(s.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_urls() {
        assert_eq!(
            classify_context("https://github.com/thtbee/nectarlink"),
            Some(ContextAction::OpenUrl("https://github.com/thtbee/nectarlink".to_string()))
        );
    }

    #[test]
    fn classifies_otp() {
        assert_eq!(
            classify_context("849201"),
            Some(ContextAction::CopyOtp("849201".to_string()))
        );
        assert_eq!(
            classify_context("582-194"),
            Some(ContextAction::CopyOtp("582194".to_string()))
        );
        assert_eq!(
            classify_context("Your verification code is 492019"),
            Some(ContextAction::CopyOtp("492019".to_string()))
        );
    }

    #[test]
    fn classifies_tracking() {
        assert_eq!(
            classify_context("1z9999999999999999"),
            Some(ContextAction::TrackPackage {
                carrier: "UPS",
                tracking_number: "1Z9999999999999999".to_string(),
            })
        );
    }

    #[test]
    fn classifies_phone() {
        assert_eq!(
            classify_context("+1 (555) 234-5678"),
            Some(ContextAction::CallPhone("+1 (555) 234-5678".to_string()))
        );
    }

    #[test]
    fn ignores_regular_text() {
        assert_eq!(classify_context("Regular sentence without actionable tokens."), None);
        assert_eq!(classify_context(""), None);
    }
}
