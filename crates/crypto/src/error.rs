// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("HKDF expand failed: output length too large")]
    HkdfLength,

    #[error("AEAD encryption failed")]
    AeadEncrypt,

    #[error("AEAD decryption failed: authentication tag mismatch")]
    AeadDecrypt,

    #[error("invalid key material: {0}")]
    InvalidKey(&'static str),

    #[error("signature verification failed")]
    SignatureInvalid,

    #[error("pairing transcript mismatch")]
    TranscriptMismatch,
}
