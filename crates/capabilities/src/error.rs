// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;
use protocol::CapabilityId;

#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("Capability {0} is not available on this host OS")]
    OsUnavailable(CapabilityId),

    #[error("Application lacks required system permission for capability {0}")]
    AppPermissionDenied(CapabilityId),

    #[error("Peer authorization denied for capability {0}")]
    PeerDenied(CapabilityId),

    #[error("Capability {0} was not negotiated for this session")]
    NotNegotiated(CapabilityId),

    #[error("Capability advertisement exceeds maximum allowed entries ({0})")]
    TooManyEntries(usize),
}
