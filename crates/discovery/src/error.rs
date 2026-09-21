// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("mDNS error: {0}")]
    Mdns(#[from] mdns_sd::Error),

    #[error("Failed to parse discovery properties: {0}")]
    InvalidProperty(String),

    #[error("Discovery daemon shutdown")]
    Shutdown,
}
