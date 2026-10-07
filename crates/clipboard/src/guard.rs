// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

const SENSITIVE_FORMATS: &[&str] = &[
    "ExcludeClipboardContentFromMonitorProcessing",
    "Clipboard Viewer Ignore",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitiveGuardDecision {
    Allow,
    DropSensitive,
}

pub fn evaluate_clipboard_formats<I, S>(available_formats: I) -> SensitiveGuardDecision
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    for format in available_formats {
        let name = format.as_ref();
        if SENSITIVE_FORMATS.iter().any(|&s| name.eq_ignore_ascii_case(s)) {
            return SensitiveGuardDecision::DropSensitive;
        }
    }
    SensitiveGuardDecision::Allow
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_regular_formats() {
        assert_eq!(
            evaluate_clipboard_formats(["CF_UNICODETEXT", "CF_TEXT"]),
            SensitiveGuardDecision::Allow
        );
    }

    #[test]
    fn drops_sensitive_formats() {
        assert_eq!(
            evaluate_clipboard_formats(["CF_UNICODETEXT", "Clipboard Viewer Ignore"]),
            SensitiveGuardDecision::DropSensitive
        );
        assert_eq!(
            evaluate_clipboard_formats(["ExcludeClipboardContentFromMonitorProcessing"]),
            SensitiveGuardDecision::DropSensitive
        );
        assert_eq!(
            evaluate_clipboard_formats(["clipboard viewer ignore"]),
            SensitiveGuardDecision::DropSensitive
        );
    }
}
