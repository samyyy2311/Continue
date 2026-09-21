// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};

use crypto::pairing::{build_initiator_transcript, InitiatorTranscriptInputs};
use limits::{SESSION_TOKEN_LEN, SHA256_DIGEST_LEN};
use crate::error::PairingError;

pub const QR_FORMAT_VERSION: u8 = 0x01;
pub const QR_ROLE_INITIATOR: u8 = 0x01;
const QR_FIXED_PREFIX_LEN: usize = 1 + 1 + 32 + 32 + SHA256_DIGEST_LEN + SESSION_TOKEN_LEN + 64;

/// Parsed pairing QR code payload.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct QrPayload {
    pub format_version: u8,
    pub role: u8,
    pub identity_pubkey: [u8; 32],
    pub x25519_ephemeral: [u8; 32],
    pub transport_spki_hash: [u8; 32],
    pub session_token: [u8; 16],
    pub signature: [u8; 64],
    pub endpoint: String,
}

impl QrPayload {
    /// Encode payload into a URL-safe Base64 string without padding.
    pub fn encode(&self) -> String {
        let endpoint_bytes = self.endpoint.as_bytes();
        let total_len = QR_FIXED_PREFIX_LEN + 2 + endpoint_bytes.len();
        let mut raw = Vec::with_capacity(total_len);

        raw.push(self.format_version);
        raw.push(self.role);
        raw.extend_from_slice(&self.identity_pubkey);
        raw.extend_from_slice(&self.x25519_ephemeral);
        raw.extend_from_slice(&self.transport_spki_hash);
        raw.extend_from_slice(&self.session_token);
        raw.extend_from_slice(&self.signature);
        raw.extend_from_slice(&(endpoint_bytes.len() as u16).to_be_bytes());
        raw.extend_from_slice(endpoint_bytes);

        URL_SAFE_NO_PAD.encode(&raw)
    }

    /// Decode payload from a URL-safe Base64 string.
    pub fn decode(encoded: &str) -> Result<Self, PairingError> {
        let raw = URL_SAFE_NO_PAD
            .decode(encoded.trim())
            .map_err(|e| PairingError::InvalidQr(format!("Base64 decode failed: {e}")))?;

        if raw.len() < QR_FIXED_PREFIX_LEN + 2 {
            return Err(PairingError::InvalidQr("Payload too short".to_string()));
        }

        let format_version = raw[0];
        if format_version != QR_FORMAT_VERSION {
            return Err(PairingError::InvalidQr(format!(
                "Unsupported QR format version {format_version}"
            )));
        }

        let role = raw[1];
        if role != QR_ROLE_INITIATOR {
            return Err(PairingError::InvalidQr(format!("Unexpected role {role}")));
        }

        let mut offset = 2;
        let mut identity_pubkey = [0u8; 32];
        identity_pubkey.copy_from_slice(&raw[offset..offset + 32]);
        offset += 32;

        let mut x25519_ephemeral = [0u8; 32];
        x25519_ephemeral.copy_from_slice(&raw[offset..offset + 32]);
        offset += 32;

        let mut transport_spki_hash = [0u8; 32];
        transport_spki_hash.copy_from_slice(&raw[offset..offset + 32]);
        offset += 32;

        let mut session_token = [0u8; 16];
        session_token.copy_from_slice(&raw[offset..offset + 16]);
        offset += 16;

        let mut signature = [0u8; 64];
        signature.copy_from_slice(&raw[offset..offset + 64]);
        offset += 64;

        let endpoint_len = u16::from_be_bytes([raw[offset], raw[offset + 1]]) as usize;
        offset += 2;

        if raw.len() != offset + endpoint_len {
            return Err(PairingError::InvalidQr("Malformed endpoint length".to_string()));
        }

        let endpoint = std::str::from_utf8(&raw[offset..offset + endpoint_len])
            .map_err(|_| PairingError::InvalidQr("Non-UTF8 endpoint".to_string()))?
            .to_string();

        Ok(Self {
            format_version,
            role,
            identity_pubkey,
            x25519_ephemeral,
            transport_spki_hash,
            session_token,
            signature,
            endpoint,
        })
    }

    /// Verify the initiator's Ed25519 signature over the canonical initiator pairing transcript.
    pub fn verify_signature(&self, protocol_version: u32) -> Result<(), PairingError> {
        let verifying_key = VerifyingKey::from_bytes(&self.identity_pubkey)
            .map_err(|_| PairingError::InvalidQr("Invalid Ed25519 public key in QR".to_string()))?;

        let transcript = build_initiator_transcript(&InitiatorTranscriptInputs {
            qr_format_version: self.format_version,
            protocol_version,
            identity_pubkey: &self.identity_pubkey,
            x25519_ephemeral: &self.x25519_ephemeral,
            session_token: &self.session_token,
            transport_spki_hash: &self.transport_spki_hash,
        });

        let sig = Signature::from_bytes(&self.signature);
        verifying_key
            .verify_strict(&transcript, &sig)
            .map_err(|_| PairingError::SignatureInvalid)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    #[test]
    fn qr_encode_decode_and_verify() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let pubkey_bytes = verifying_key.to_bytes();

        let x25519_eph = [0x42u8; 32];
        let spki_hash = [0x11u8; 32];
        let token = [0x99u8; 16];
        let endpoint = "192.168.1.50:4433".to_string();

        let transcript = build_initiator_transcript(&InitiatorTranscriptInputs {
            qr_format_version: QR_FORMAT_VERSION,
            protocol_version: 1,
            identity_pubkey: &pubkey_bytes,
            x25519_ephemeral: &x25519_eph,
            session_token: &token,
            transport_spki_hash: &spki_hash,
        });

        let sig = ed25519_dalek::Signer::sign(&signing_key, &transcript);

        let qr = QrPayload {
            format_version: QR_FORMAT_VERSION,
            role: QR_ROLE_INITIATOR,
            identity_pubkey: pubkey_bytes,
            x25519_ephemeral: x25519_eph,
            transport_spki_hash: spki_hash,
            session_token: token,
            signature: sig.to_bytes(),
            endpoint,
        };

        let encoded = qr.encode();
        let decoded = QrPayload::decode(&encoded).unwrap();
        assert_eq!(qr, decoded);
        assert!(decoded.verify_signature(1).is_ok());
    }
}
