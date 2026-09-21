// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

use crate::error::TransportError;
use crate::spki::compute_spki_hash;
use crate::verifier::{PairingClientCertVerifier, PinnedClientCertVerifier, PinnedServerCertVerifier};

/// Transport keypair and self-signed TLS certificate.
pub struct TransportCertificate {
    pub cert_der: CertificateDer<'static>,
    pub key_der: PrivateKeyDer<'static>,
    pub spki_hash: [u8; 32],
}

impl TransportCertificate {
    /// Generate a new ECDSA P-256 transport keypair and self-signed TLS certificate.
    pub fn generate() -> Result<Self, TransportError> {
        let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
        let spki_bytes = key_pair.public_key_der();
        let spki_hash = compute_spki_hash(&spki_bytes);

        let mut params = CertificateParams::new(vec!["continue-device".to_string()])?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "continue-device");
        params.distinguished_name = dn;

        let cert = params.self_signed(&key_pair)?;
        let cert_der = CertificateDer::from(cert.der().to_vec());
        let key_der = PrivateKeyDer::Pkcs8(key_pair.serialize_der().into());

        Ok(Self {
            cert_der,
            key_der,
            spki_hash,
        })
    }

    pub fn build_pinned_client_tls(
        &self,
        expected_server_spki_hash: [u8; 32],
    ) -> Result<rustls::ClientConfig, TransportError> {
        let verifier = Arc::new(PinnedServerCertVerifier::new(expected_server_spki_hash));
        let config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_client_auth_cert(vec![self.cert_der.clone()], self.key_der.clone_key())
            .map_err(TransportError::Tls)?;
        Ok(config)
    }

    pub fn build_pairing_server_tls(
        &self,
        recorded_spki_hash: Arc<Mutex<Option<[u8; 32]>>>,
    ) -> Result<rustls::ServerConfig, TransportError> {
        let verifier = Arc::new(PairingClientCertVerifier::new(recorded_spki_hash));
        let config = rustls::ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(vec![self.cert_der.clone()], self.key_der.clone_key())
            .map_err(TransportError::Tls)?;
        Ok(config)
    }

    pub fn build_pinned_server_tls(
        &self,
        expected_client_spki_hash: [u8; 32],
    ) -> Result<rustls::ServerConfig, TransportError> {
        let verifier = Arc::new(PinnedClientCertVerifier::new(expected_client_spki_hash));
        let config = rustls::ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(vec![self.cert_der.clone()], self.key_der.clone_key())
            .map_err(TransportError::Tls)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spki::extract_and_hash_spki;

    #[test]
    fn certificate_generation_and_spki_consistency() {
        let cert = TransportCertificate::generate().unwrap();
        let extracted_hash = extract_and_hash_spki(&cert.cert_der).unwrap();
        assert_eq!(cert.spki_hash, extracted_hash);
    }
}
