// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, DistinguishedName, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};

use crate::spki::extract_and_hash_spki;

fn default_verify_tls12_signature(
    message: &[u8],
    cert: &CertificateDer<'_>,
    dss: &DigitallySignedStruct,
) -> Result<HandshakeSignatureValid, RustlsError> {
    rustls::crypto::ring::default_provider()
        .signature_verification_algorithms
        .verify_tls12_signature(message, cert, dss)
}

fn default_verify_tls13_signature(
    message: &[u8],
    cert: &CertificateDer<'_>,
    dss: &DigitallySignedStruct,
) -> Result<HandshakeSignatureValid, RustlsError> {
    rustls::crypto::ring::default_provider()
        .signature_verification_algorithms
        .verify_tls13_signature(message, cert, dss)
}

fn default_supported_schemes() -> Vec<SignatureScheme> {
    rustls::crypto::ring::default_provider()
        .signature_verification_algorithms
        .supported_schemes()
}

/// Server certificate verifier that strictly pins a known transport SPKI hash.
///
/// Used by:
/// 1. The pairing responder connecting to the initiator (matching SPKI hash from QR).
/// 2. Any client reconnecting to a trusted peer post-pairing.
#[derive(Debug)]
pub struct PinnedServerCertVerifier {
    pub expected_spki_hash: [u8; 32],
}

impl PinnedServerCertVerifier {
    pub fn new(expected_spki_hash: [u8; 32]) -> Self {
        Self { expected_spki_hash }
    }
}

impl ServerCertVerifier for PinnedServerCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        let actual_hash = extract_and_hash_spki(end_entity.as_ref())
            .map_err(|_| RustlsError::InvalidCertificate(rustls::CertificateError::BadEncoding))?;

        if actual_hash != self.expected_spki_hash {
            return Err(RustlsError::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ));
        }

        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        default_verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        default_verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        default_supported_schemes()
    }
}

/// Client certificate verifier used by the initiator during pairing.
///
/// The initiator accepts the unknown peer's certificate and records its SPKI hash
/// so it can be verified against the signed pairing transcript later.
#[derive(Debug, Clone)]
pub struct PairingClientCertVerifier {
    pub recorded_spki_hash: Arc<Mutex<Option<[u8; 32]>>>,
}

impl PairingClientCertVerifier {
    pub fn new(recorded_spki_hash: Arc<Mutex<Option<[u8; 32]>>>) -> Self {
        Self { recorded_spki_hash }
    }
}

impl ClientCertVerifier for PairingClientCertVerifier {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, RustlsError> {
        let actual_hash = extract_and_hash_spki(end_entity.as_ref())
            .map_err(|_| RustlsError::InvalidCertificate(rustls::CertificateError::BadEncoding))?;

        if let Ok(mut guard) = self.recorded_spki_hash.lock() {
            *guard = Some(actual_hash);
        }

        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        default_verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        default_verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        default_supported_schemes()
    }
}

/// Client certificate verifier that strictly pins a known transport SPKI hash.
///
/// Used by a server receiving a connection from a trusted peer post-pairing.
#[derive(Debug)]
pub struct PinnedClientCertVerifier {
    pub expected_spki_hash: [u8; 32],
}

impl PinnedClientCertVerifier {
    pub fn new(expected_spki_hash: [u8; 32]) -> Self {
        Self { expected_spki_hash }
    }
}

impl ClientCertVerifier for PinnedClientCertVerifier {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, RustlsError> {
        let actual_hash = extract_and_hash_spki(end_entity.as_ref())
            .map_err(|_| RustlsError::InvalidCertificate(rustls::CertificateError::BadEncoding))?;

        if actual_hash != self.expected_spki_hash {
            return Err(RustlsError::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ));
        }

        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        default_verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        default_verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        default_supported_schemes()
    }
}
