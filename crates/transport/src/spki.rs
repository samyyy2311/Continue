// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use sha2::{Digest, Sha256};
use crate::error::TransportError;

fn parse_der_length(input: &[u8]) -> Option<(usize, usize)> {
    if input.is_empty() {
        return None;
    }
    let first = input[0];
    if first < 0x80 {
        Some((first as usize, 1))
    } else {
        let num_bytes = (first & 0x7F) as usize;
        if num_bytes == 0 || num_bytes > 4 || input.len() < 1 + num_bytes {
            return None;
        }
        let mut len = 0usize;
        for &b in &input[1..=num_bytes] {
            len = (len << 8) | (b as usize);
        }
        Some((len, 1 + num_bytes))
    }
}

fn parse_der_tlv(input: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    if input.is_empty() {
        return None;
    }
    let tag = input[0];
    let (len, header_len) = parse_der_length(&input[1..])?;
    let total_len = 1 + header_len + len;
    if input.len() < total_len {
        return None;
    }
    let value = &input[1 + header_len..total_len];
    let rest = &input[total_len..];
    Some((tag, value, rest))
}

/// Extract raw SubjectPublicKeyInfo (SPKI) bytes from an X.509 certificate in DER format.
pub fn extract_spki_bytes(cert_der: &[u8]) -> Result<&[u8], TransportError> {
    let (tag, cert_body, _) = parse_der_tlv(cert_der).ok_or(TransportError::SpkiExtractionFailed)?;
    if tag != 0x30 {
        return Err(TransportError::SpkiExtractionFailed);
    }

    let (tag, mut tbs_fields, _) = parse_der_tlv(cert_body).ok_or(TransportError::SpkiExtractionFailed)?;
    if tag != 0x30 {
        return Err(TransportError::SpkiExtractionFailed);
    }

    // Skip optional version [0]
    if let Some((0xA0, _, rest)) = parse_der_tlv(tbs_fields) {
        tbs_fields = rest;
    }

    // Skip serialNumber
    let (_, _, rest) = parse_der_tlv(tbs_fields).ok_or(TransportError::SpkiExtractionFailed)?;
    tbs_fields = rest;

    // Skip signature algorithm
    let (_, _, rest) = parse_der_tlv(tbs_fields).ok_or(TransportError::SpkiExtractionFailed)?;
    tbs_fields = rest;

    // Skip issuer
    let (_, _, rest) = parse_der_tlv(tbs_fields).ok_or(TransportError::SpkiExtractionFailed)?;
    tbs_fields = rest;

    // Skip validity
    let (_, _, rest) = parse_der_tlv(tbs_fields).ok_or(TransportError::SpkiExtractionFailed)?;
    tbs_fields = rest;

    // Skip subject
    let (_, _, rest) = parse_der_tlv(tbs_fields).ok_or(TransportError::SpkiExtractionFailed)?;
    tbs_fields = rest;

    // SubjectPublicKeyInfo must be next (SEQUENCE 0x30)
    if tbs_fields.is_empty() || tbs_fields[0] != 0x30 {
        return Err(TransportError::SpkiExtractionFailed);
    }

    let (len, header_len) = parse_der_length(&tbs_fields[1..]).ok_or(TransportError::SpkiExtractionFailed)?;
    let total_spki_len = 1 + header_len + len;
    if tbs_fields.len() < total_spki_len {
        return Err(TransportError::SpkiExtractionFailed);
    }

    Ok(&tbs_fields[..total_spki_len])
}

/// Compute SHA-256 digest of SubjectPublicKeyInfo bytes.
pub fn compute_spki_hash(spki_bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(spki_bytes);
    hasher.finalize().into()
}

/// Extract SPKI from an X.509 DER certificate and return its SHA-256 hash.
pub fn extract_and_hash_spki(cert_der: &[u8]) -> Result<[u8; 32], TransportError> {
    let spki = extract_spki_bytes(cert_der)?;
    Ok(compute_spki_hash(spki))
}
