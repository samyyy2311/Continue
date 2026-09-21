// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use limits::MAX_CAPABILITY_ENTRIES;
use protocol::v1::CapabilityEntry;
use protocol::CapabilityId;

use crate::error::CapabilityError;

/// Safely compute the negotiated intersection of local and remote capability sets.
///
/// Rules:
/// - Caps total entries to `MAX_CAPABILITY_ENTRIES` to prevent memory exhaustion.
/// - Matches by `CapabilityId`.
/// - Selects `min(local_version, remote_version)` as the active version.
/// - Skips capabilities where negotiated version == 0.
/// - Safely ignores unknown capability IDs received from remote peers without erroring.
pub fn negotiate_capabilities(
    local_supported: &[CapabilityEntry],
    remote_advertised: &[CapabilityEntry],
) -> Result<Vec<CapabilityEntry>, CapabilityError> {
    if remote_advertised.len() > MAX_CAPABILITY_ENTRIES {
        return Err(CapabilityError::TooManyEntries(remote_advertised.len()));
    }

    let mut remote_map = HashMap::new();
    for entry in remote_advertised {
        remote_map.insert(entry.id, entry.version);
    }

    let mut negotiated = Vec::new();
    for local in local_supported {
        if let Some(&remote_version) = remote_map.get(&local.id) {
            let active_version = local.version.min(remote_version);
            if active_version > 0 {
                negotiated.push(CapabilityEntry {
                    id: local.id,
                    version: active_version,
                });
            }
        }
    }

    Ok(negotiated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiation_intersection_and_unknown_handling() {
        let local = vec![
            CapabilityEntry {
                id: CapabilityId::FILE_TRANSFER.raw(),
                version: 2,
            },
            CapabilityEntry {
                id: CapabilityId::CLIPBOARD.raw(),
                version: 1,
            },
        ];

        let remote = vec![
            CapabilityEntry {
                id: CapabilityId::FILE_TRANSFER.raw(),
                version: 1, // Peer supports version 1
            },
            CapabilityEntry {
                id: 999, // Unknown future capability
                version: 5,
            },
        ];

        let result = negotiate_capabilities(&local, &remote).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, CapabilityId::FILE_TRANSFER.raw());
        assert_eq!(result[0].version, 1);
    }

    #[test]
    fn negotiation_rejects_oversized_advertisements() {
        let local = vec![];
        let remote: Vec<CapabilityEntry> = (0..MAX_CAPABILITY_ENTRIES + 1)
            .map(|i| CapabilityEntry {
                id: i as u32,
                version: 1,
            })
            .collect();

        assert!(matches!(
            negotiate_capabilities(&local, &remote),
            Err(CapabilityError::TooManyEntries(_))
        ));
    }
}
