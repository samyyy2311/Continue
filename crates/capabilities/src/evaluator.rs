// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use protocol::CapabilityId;
use crate::error::CapabilityError;

/// Evaluation query context for checking whether a capability operation is authorized.
pub struct CapabilityQuery {
    pub capability: CapabilityId,
    pub is_os_available: bool,
    pub is_app_permitted: bool,
    pub is_peer_authorized: bool,
    pub negotiated_session_capabilities: HashSet<CapabilityId>,
}

/// Evaluates capability permission across the four required independent security boundaries:
///
/// 1. OS Availability: Is the platform hardware / OS API supported?
/// 2. App Permission: Does this application process have OS permission?
/// 3. Peer Authorization: Has the user granted this peer permission?
/// 4. Session Authorization: Was this capability negotiated for this active session?
pub fn evaluate_capability(query: &CapabilityQuery) -> Result<(), CapabilityError> {
    if !query.is_os_available {
        return Err(CapabilityError::OsUnavailable(query.capability));
    }

    if !query.is_app_permitted {
        return Err(CapabilityError::AppPermissionDenied(query.capability));
    }

    if !query.is_peer_authorized {
        return Err(CapabilityError::PeerDenied(query.capability));
    }

    if !query.negotiated_session_capabilities.contains(&query.capability) {
        return Err(CapabilityError::NotNegotiated(query.capability));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_layer_evaluator_all_pass() {
        let mut caps = HashSet::new();
        caps.insert(CapabilityId::FILE_TRANSFER);

        let query = CapabilityQuery {
            capability: CapabilityId::FILE_TRANSFER,
            is_os_available: true,
            is_app_permitted: true,
            is_peer_authorized: true,
            negotiated_session_capabilities: caps,
        };

        assert!(evaluate_capability(&query).is_ok());
    }

    #[test]
    fn four_layer_evaluator_fails_if_any_layer_fails() {
        let mut caps = HashSet::new();
        caps.insert(CapabilityId::FILE_TRANSFER);

        // OS unavailable
        let q1 = CapabilityQuery {
            capability: CapabilityId::FILE_TRANSFER,
            is_os_available: false,
            is_app_permitted: true,
            is_peer_authorized: true,
            negotiated_session_capabilities: caps.clone(),
        };
        assert!(matches!(evaluate_capability(&q1), Err(CapabilityError::OsUnavailable(_))));

        // App permission denied
        let q2 = CapabilityQuery {
            capability: CapabilityId::FILE_TRANSFER,
            is_os_available: true,
            is_app_permitted: false,
            is_peer_authorized: true,
            negotiated_session_capabilities: caps.clone(),
        };
        assert!(matches!(evaluate_capability(&q2), Err(CapabilityError::AppPermissionDenied(_))));

        // Peer denied
        let q3 = CapabilityQuery {
            capability: CapabilityId::FILE_TRANSFER,
            is_os_available: true,
            is_app_permitted: true,
            is_peer_authorized: false,
            negotiated_session_capabilities: caps.clone(),
        };
        assert!(matches!(evaluate_capability(&q3), Err(CapabilityError::PeerDenied(_))));

        // Not in negotiated session set
        let q4 = CapabilityQuery {
            capability: CapabilityId::FILE_TRANSFER,
            is_os_available: true,
            is_app_permitted: true,
            is_peer_authorized: true,
            negotiated_session_capabilities: HashSet::new(),
        };
        assert!(matches!(evaluate_capability(&q4), Err(CapabilityError::NotNegotiated(_))));
    }
}
