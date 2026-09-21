// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;
use limits::MAX_TRANSFER_FILENAME_BYTES;
use crate::error::TransferError;

/// Sanitize and validate a filename proposed by a remote peer.
///
/// Strips directory components and traversal tokens (`..`), disallows control characters
/// and Windows reserved device names, and bounds length to `MAX_TRANSFER_FILENAME_BYTES`.
pub fn sanitize_filename(raw_name: &str) -> Result<String, TransferError> {
    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return Err(TransferError::InvalidFilename("Filename cannot be empty".to_string()));
    }

    if trimmed.len() > MAX_TRANSFER_FILENAME_BYTES {
        return Err(TransferError::InvalidFilename(format!(
            "Filename exceeds max byte length ({})",
            MAX_TRANSFER_FILENAME_BYTES
        )));
    }

    // Extract filename portion from any path components (supporting both / and \)
    let normalized = trimmed.replace('\\', "/");
    let path = Path::new(&normalized);
    let filename_os = path
        .file_name()
        .ok_or_else(|| TransferError::InvalidFilename("Invalid file name component".to_string()))?;

    let filename_str = filename_os
        .to_str()
        .ok_or_else(|| TransferError::InvalidFilename("Filename must be valid UTF-8".to_string()))?;

    if filename_str == "." || filename_str == ".." {
        return Err(TransferError::InvalidFilename("Relative traversal disallowed".to_string()));
    }

    // Filter control characters and forbidden characters: < > : " / \ | ? * and null bytes
    let clean: String = filename_str
        .chars()
        .filter(|&c| !c.is_control() && !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0'))
        .collect();

    let clean = clean.trim();
    if clean.is_empty() {
        return Err(TransferError::InvalidFilename(
            "Filename contains only invalid characters".to_string(),
        ));
    }

    // Check Windows reserved names
    let stem = clean.split('.').next().unwrap_or(clean);
    let upper_stem = stem.to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL",
        "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if reserved.contains(&upper_stem.as_str()) {
        return Err(TransferError::InvalidFilename(format!(
            "Filename uses reserved system device name: {upper_stem}"
        )));
    }

    Ok(clean.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_filename() {
        assert_eq!(sanitize_filename("photo.jpg").unwrap(), "photo.jpg");
        assert_eq!(sanitize_filename("my document (1).pdf").unwrap(), "my document (1).pdf");
    }

    #[test]
    fn strips_directory_traversal() {
        assert_eq!(sanitize_filename("../../secret.txt").unwrap(), "secret.txt");
        assert_eq!(sanitize_filename("C:\\Windows\\System32\\cmd.exe").unwrap(), "cmd.exe");
        assert_eq!(sanitize_filename("/etc/passwd").unwrap(), "passwd");
    }

    #[test]
    fn rejects_traversal_only() {
        assert!(sanitize_filename("..").is_err());
        assert!(sanitize_filename(".").is_err());
        assert!(sanitize_filename("../..").is_err());
        assert!(sanitize_filename("").is_err());
    }

    #[test]
    fn rejects_reserved_names() {
        assert!(sanitize_filename("NUL").is_err());
        assert!(sanitize_filename("con.txt").is_err());
        assert!(sanitize_filename("aux.png").is_err());
        assert!(sanitize_filename("COM1.bin").is_err());
    }

    #[test]
    fn filters_illegal_characters() {
        assert_eq!(sanitize_filename("bad<name>:test*.txt").unwrap(), "badnametest.txt");
    }
}
