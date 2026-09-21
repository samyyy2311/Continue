// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Pairing transcript construction and HMAC-SHA256 key confirmation MACs.
//!
//! The transcript is a canonical byte sequence over all pairing parameters.
//! Both the Ed25519 pairing signatures and the PairConfirm MACs are computed
//! over this transcript, binding all parameters together.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::error::CryptoError;

type HmacSha256 = Hmac<Sha256>;

// Domain separators â€” ASCII, fixed length, version-tagged.
// Role separation prevents reflection attacks.
const DOMAIN_INIT: &[u8] = b"PAIRING_V1_INIT";
const DOMAIN_RESP: &[u8] = b"PAIRING_V1_RESP";

/// Labels for role-separated PairConfirm MACs.
const CONFIRM_INIT_LABEL: &[u8] = b"CONFIRM_INIT_V1";
const CONFIRM_RESP_LABEL: &[u8] = b"CONFIRM_RESP_V1";

/// All inputs needed to construct the initiator pairing transcript.
pub struct InitiatorTranscriptInputs<'a> {
    pub qr_format_version: u8,
    pub protocol_version: u32,
    /// Initiator's persistent Ed25519 public key (32 bytes).
    pub identity_pubkey: &'a [u8; 32],
    /// Initiator's ephemeral X25519 public key (32 bytes).
    pub x25519_ephemeral: &'a [u8; 32],
    /// Pairing session token (SESSION_TOKEN_LEN bytes).
    pub session_token: &'a [u8],
    /// SHA-256 of initiator transport SPKI (SHA256_DIGEST_LEN bytes).
    pub transport_spki_hash: &'a [u8; 32],
}

/// All inputs needed to construct the responder pairing transcript.
pub struct ResponderTranscriptInputs<'a> {
    pub qr_format_version: u8,
    pub protocol_version: u32,
    /// Responder's persistent Ed25519 public key (32 bytes).
    pub identity_pubkey: &'a [u8; 32],
    /// Responder's ephemeral X25519 public key (32 bytes).
    pub x25519_ephemeral: &'a [u8; 32],
    /// Session token echoed from the initiator (SESSION_TOKEN_LEN bytes).
    pub session_token: &'a [u8],
    /// SHA-256 of responder transport SPKI (SHA256_DIGEST_LEN bytes).
    pub transport_spki_hash: &'a [u8; 32],
}

/// Build the canonical initiator transcript byte sequence.
///
/// Transcript layout:
///   "PAIRING_V1_INIT" (15)
///   || qr_format_version (u8, 1 byte)
///   || protocol_version  (u32 LE, 4 bytes)
///   || 0x01              (role byte, 1 byte)
///   || identity_pubkey   (32 bytes)
///   || x25519_ephemeral  (32 bytes)
///   || session_token     (16 bytes)
///   || transport_spki_hash (32 bytes)
pub fn build_initiator_transcript(inputs: &InitiatorTranscriptInputs<'_>) -> Vec<u8> {
    let mut t = Vec::with_capacity(15 + 1 + 4 + 1 + 32 + 32 + 16 + 32);
    t.extend_from_slice(DOMAIN_INIT);
    t.push(inputs.qr_format_version);
    t.extend_from_slice(&inputs.protocol_version.to_le_bytes());
    t.push(0x01); // role: initiator
    t.extend_from_slice(inputs.identity_pubkey);
    t.extend_from_slice(inputs.x25519_ephemeral);
    t.extend_from_slice(inputs.session_token);
    t.extend_from_slice(inputs.transport_spki_hash);
    t
}

/// Build the canonical responder transcript byte sequence.
///
/// Same layout as the initiator transcript with "PAIRING_V1_RESP" and role 0x02.
pub fn build_responder_transcript(inputs: &ResponderTranscriptInputs<'_>) -> Vec<u8> {
    let mut t = Vec::with_capacity(15 + 1 + 4 + 1 + 32 + 32 + 16 + 32);
    t.extend_from_slice(DOMAIN_RESP);
    t.push(inputs.qr_format_version);
    t.extend_from_slice(&inputs.protocol_version.to_le_bytes());
    t.push(0x02); // role: responder
    t.extend_from_slice(inputs.identity_pubkey);
    t.extend_from_slice(inputs.x25519_ephemeral);
    t.extend_from_slice(inputs.session_token);
    t.extend_from_slice(inputs.transport_spki_hash);
    t
}

/// Build the full pairing transcript used for PairConfirm MACs.
///
/// `full_transcript = initiator_transcript || responder_transcript`
pub fn build_full_transcript(initiator: &[u8], responder: &[u8]) -> Vec<u8> {
    let mut t = Vec::with_capacity(initiator.len() + responder.len());
    t.extend_from_slice(initiator);
    t.extend_from_slice(responder);
    t
}

/// Compute the initiator's PairConfirm MAC.
///
/// `HMAC-SHA256(confirmation_key, "CONFIRM_INIT_V1" || full_transcript)`
pub fn confirm_mac_initiator(
    confirmation_key: &Zeroizing<[u8; 32]>,
    full_transcript: &[u8],
) -> Result<[u8; 32], CryptoError> {
    compute_confirm_mac(confirmation_key, CONFIRM_INIT_LABEL, full_transcript)
}

/// Compute the responder's PairConfirm MAC.
///
/// `HMAC-SHA256(confirmation_key, "CONFIRM_RESP_V1" || full_transcript)`
pub fn confirm_mac_responder(
    confirmation_key: &Zeroizing<[u8; 32]>,
    full_transcript: &[u8],
) -> Result<[u8; 32], CryptoError> {
    compute_confirm_mac(confirmation_key, CONFIRM_RESP_LABEL, full_transcript)
}

fn compute_confirm_mac(
    key: &Zeroizing<[u8; 32]>,
    label: &[u8],
    transcript: &[u8],
) -> Result<[u8; 32], CryptoError> {
    let mut mac =
        HmacSha256::new_from_slice(key.as_ref()).map_err(|_| CryptoError::InvalidKey("HMAC key"))?;
    mac.update(label);
    mac.update(transcript);
    let result = mac.finalize().into_bytes();
    Ok(result.into())
}

/// Verify a PairConfirm MAC in constant time.
///
/// Returns `Err(CryptoError::TranscriptMismatch)` on failure.
pub fn verify_confirm_mac(expected: &[u8; 32], actual: &[u8; 32]) -> Result<(), CryptoError> {
    use subtle::ConstantTimeEq;
    if expected.ct_eq(actual).into() {
        Ok(())
    } else {
        Err(CryptoError::TranscriptMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_32() -> [u8; 32] {
        [0u8; 32]
    }

    fn dummy_token() -> [u8; 16] {
        [1u8; 16]
    }

    #[test]
    fn transcripts_differ_by_role() {
        let init_inputs = InitiatorTranscriptInputs {
            qr_format_version: 1,
            protocol_version: 1,
            identity_pubkey: &dummy_32(),
            x25519_ephemeral: &dummy_32(),
            session_token: &dummy_token(),
            transport_spki_hash: &dummy_32(),
        };
        let resp_inputs = ResponderTranscriptInputs {
            qr_format_version: 1,
            protocol_version: 1,
            identity_pubkey: &dummy_32(),
            x25519_ephemeral: &dummy_32(),
            session_token: &dummy_token(),
            transport_spki_hash: &dummy_32(),
        };
        let t_init = build_initiator_transcript(&init_inputs);
        let t_resp = build_responder_transcript(&resp_inputs);
        // Same key material but different role bytes and domain separators.
        assert_ne!(t_init, t_resp);
    }

    #[test]
    fn confirm_macs_differ_by_role() {
        let key = Zeroizing::new([0u8; 32]);
        let transcript = vec![0u8; 64];
        let mac_init = confirm_mac_initiator(&key, &transcript).unwrap();
        let mac_resp = confirm_mac_responder(&key, &transcript).unwrap();
        assert_ne!(mac_init, mac_resp);
    }

    #[test]
    fn confirm_mac_verify_ok() {
        let key = Zeroizing::new([0u8; 32]);
        let transcript = vec![1u8; 64];
        let mac = confirm_mac_initiator(&key, &transcript).unwrap();
        verify_confirm_mac(&mac, &mac).unwrap();
    }

    #[test]
    fn confirm_mac_verify_tampered() {
        let key = Zeroizing::new([0u8; 32]);
        let transcript = vec![1u8; 64];
        let mac = confirm_mac_initiator(&key, &transcript).unwrap();
        let mut bad = mac;
        bad[0] ^= 0xff;
        assert!(verify_confirm_mac(&mac, &bad).is_err());
    }
}
