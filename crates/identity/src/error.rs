// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecretStoreError {
    #[error("secret store unavailable: {0}")]
    Unavailable(String),

    #[error("key not found: {label}")]
    NotFound { label: String },

    #[error("I/O error accessing secret store: {0}")]
    Io(String),

    #[error("secret store rejected the operation: {0}")]
    Rejected(String),
}

#[derive(Debug, Error)]
pub enum SignerError {
    #[error("signing key not loaded")]
    NotLoaded,

    #[error("hardware signer fault: {0}")]
    HardwareFault(String),

    #[error("signer operation rejected by OS/platform: {0}")]
    PlatformRejected(String),
}

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("secret store error: {0}")]
    Store(#[from] SecretStoreError),

    #[error("signer error: {0}")]
    Signer(#[from] SignerError),

    #[error("stored seed has invalid length {0}; expected 32")]
    InvalidSeedLength(usize),
}
