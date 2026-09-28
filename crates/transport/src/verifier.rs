// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{DigitallySignedStruct, DistinguishedName, Error as RustlsError, SignatureScheme};

use crate::spki::extract_and_hash_spki;

fn default_verify_tls12_signature(
    message: &[u8],
    cert: &CertificateDer<'_>,
    dss: &DigitallySignedStruct,
) -> Result<HandshakeSignatureValid, RustlsError> {
    rustls::crypto::verify_tls12_signature(
        message,
        cert,
        dss,
        &rustls::crypto::ring::default_provider().signature_verification_algorithms,
    )
}

fn default_verify_tls13_signature(
    message: &[u8],
    cert: &CertificateDer<'_>,
    dss: &DigitallySignedStruct,
) -> Result<HandshakeSignatureValid, RustlsError> {
    rustls::crypto::verify_tls13_signature(
        message,
        cert,
        dss,
        &rustls::crypto::ring::default_provider().signature_verification_algorithms,
    )
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

/// Client certificate verifier that checks incoming client certificates against a set of allowed peer SPKI hashes.
#[derive(Debug, Clone)]
pub struct TrustedPeersClientCertVerifier {
    pub allowed_spki_hashes: Arc<std::sync::RwLock<std::collections::HashSet<[u8; 32]>>>,
}

impl TrustedPeersClientCertVerifier {
    pub fn new(allowed: std::collections::HashSet<[u8; 32]>) -> Self {
        Self {
            allowed_spki_hashes: Arc::new(std::sync::RwLock::new(allowed)),
        }
    }

    pub fn from_shared(
        shared: Arc<std::sync::RwLock<std::collections::HashSet<[u8; 32]>>>,
    ) -> Self {
        Self {
            allowed_spki_hashes: shared,
        }
    }
}

impl ClientCertVerifier for TrustedPeersClientCertVerifier {
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

        if let Ok(guard) = self.allowed_spki_hashes.read() {
            if guard.contains(&actual_hash) {
                return Ok(ClientCertVerified::assertion());
            }
        }

        Err(RustlsError::InvalidCertificate(
            rustls::CertificateError::ApplicationVerificationFailure,
        ))
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

/// Extract the remote peer's transport SPKI hash from an established QUIC connection.
pub fn extract_peer_spki_hash(connection: &quinn::Connection) -> Option<[u8; 32]> {
    let peer_id = connection.peer_identity()?;
    if let Some(certs) = peer_id.downcast_ref::<Vec<rustls::pki_types::CertificateDer<'static>>>() {
        certs
            .first()
            .and_then(|c| crate::spki::extract_and_hash_spki(c.as_ref()).ok())
    } else if let Some(certs) =
        peer_id.downcast_ref::<Arc<Vec<rustls::pki_types::CertificateDer<'static>>>>()
    {
        certs
            .first()
            .and_then(|c| crate::spki::extract_and_hash_spki(c.as_ref()).ok())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cert::TransportCertificate;
    use crate::endpoint::{create_client_endpoint, create_server_endpoint};

    #[test]
    fn trusted_peers_server_accepts_and_inspects_peer() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let server_cert = TransportCertificate::generate().unwrap();
            let client_cert = TransportCertificate::generate().unwrap();

            let mut allowed = std::collections::HashSet::new();
            allowed.insert(client_cert.spki_hash);
            let allowed_shared = Arc::new(std::sync::RwLock::new(allowed));

            let server_tls = server_cert
                .build_trusted_peers_server_tls(allowed_shared)
                .unwrap();
            let server_ep =
                create_server_endpoint("127.0.0.1:0".parse().unwrap(), server_tls).unwrap();
            let server_addr = server_ep.local_addr().unwrap();

            let client_tls = client_cert
                .build_pinned_client_tls(server_cert.spki_hash)
                .unwrap();
            let client_ep =
                create_client_endpoint("127.0.0.1:0".parse().unwrap(), client_tls).unwrap();

            let (tx, rx) = tokio::sync::oneshot::channel();
            tokio::spawn(async move {
                let incoming = server_ep.accept().await.unwrap();
                let conn = incoming.await.unwrap();
                let hash = extract_peer_spki_hash(&conn);
                let _ = tx.send((conn, hash));
            });

            let client_conn = client_ep
                .connect(server_addr, "continue-device")
                .unwrap()
                .await
                .unwrap();

            let (server_conn, extracted_hash) = rx.await.unwrap();
            assert_eq!(extracted_hash, Some(client_cert.spki_hash));

            client_conn.close(0u32.into(), b"done");
            server_conn.close(0u32.into(), b"done");
        });
    }
}
