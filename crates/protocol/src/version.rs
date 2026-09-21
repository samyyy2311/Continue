// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use crate::error::ProtocolError;

/// Current application protocol version.
pub const CURRENT_PROTOCOL_VERSION: u32 = 1;

/// Minimum protocol version supported by this build.
pub const MIN_SUPPORTED_PROTOCOL_VERSION: u32 = 1;

/// Maximum protocol version supported by this build.
pub const MAX_SUPPORTED_PROTOCOL_VERSION: u32 = 1;

/// Check if a given peer protocol version is supported.
pub fn is_version_supported(version: u32) -> bool {
    (MIN_SUPPORTED_PROTOCOL_VERSION..=MAX_SUPPORTED_PROTOCOL_VERSION).contains(&version)
}

/// Negotiate the protocol version to use with a peer.
///
/// Returns the negotiated version or an error if the peer's version is unsupported.
pub fn negotiate_protocol_version(peer_version: u32) -> Result<u32, ProtocolError> {
    if is_version_supported(peer_version) {
        Ok(peer_version.min(MAX_SUPPORTED_PROTOCOL_VERSION))
    } else {
        Err(ProtocolError::IncompatibleVersion {
            peer_version,
            min_supported: MIN_SUPPORTED_PROTOCOL_VERSION,
            max_supported: MAX_SUPPORTED_PROTOCOL_VERSION,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compatibility() {
        assert!(is_version_supported(1));
        assert!(!is_version_supported(0));
        assert!(!is_version_supported(2));

        assert_eq!(negotiate_protocol_version(1).unwrap(), 1);
        assert!(negotiate_protocol_version(0).is_err());
        assert!(negotiate_protocol_version(2).is_err());
    }
}
