// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PairingError {
    #[error("Invalid QR payload: {0}")]
    InvalidQr(String),

    #[error("Pairing signature verification failed")]
    SignatureInvalid,

    #[error("Session token mismatch")]
    SessionTokenMismatch,

    #[error("Session token has expired")]
    SessionTokenExpired,

    #[error("Session token was already used (replay rejected)")]
    SessionTokenReplayed,

    #[error("Transport SPKI hash mismatch")]
    SpkiMismatch,

    #[error("PairConfirm MAC verification failed")]
    MacMismatch,

    #[error("Cryptographic operation error: {0}")]
    Crypto(#[from] crypto::error::CryptoError),

    #[error("Identity signer error: {0}")]
    Signer(#[from] identity::SignerError),

    #[error("Trust store database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Transport error: {0}")]
    Transport(#[from] transport::TransportError),

    #[error("Protocol framing error: {0}")]
    Protocol(#[from] protocol::ProtocolError),
}
