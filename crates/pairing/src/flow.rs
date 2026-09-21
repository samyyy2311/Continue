// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::BytesMut;
use ed25519_dalek::{Signature, VerifyingKey};
use x25519_dalek::PublicKey as X25519PublicKey;

use crypto::hkdf::derive_pairing_keys;
use crypto::keys::EphemeralX25519;
use crypto::pairing::{
    build_full_transcript, build_initiator_transcript, build_responder_transcript,
    confirm_mac_initiator, confirm_mac_responder, verify_confirm_mac,
    InitiatorTranscriptInputs, ResponderTranscriptInputs,
};
use crypto::token::SessionToken;
use identity::{Fingerprint, IdentitySigner};
use limits::{MAX_FRAME_PAIRING_BYTES, MAX_PAIRING_SESSION_SECS};
use protocol::v1::{KeyExchangeResponse, PairConfirm};
use transport::TransportCertificate;

use crate::error::PairingError;
use crate::qr::{QrPayload, QR_FORMAT_VERSION, QR_ROLE_INITIATOR};
use crate::replay::ReplayCache;
use crate::trust_store::{TrustStore, TrustedPeer};

async fn write_msg<M: prost::Message>(
    stream: &mut quinn::SendStream,
    msg: &M,
) -> Result<(), PairingError> {
    let frame = protocol::encode_frame(msg, MAX_FRAME_PAIRING_BYTES)?;
    stream
        .write_all(&frame)
        .await
        .map_err(transport::TransportError::from)?;
    Ok(())
}

async fn read_msg<M: prost::Message + Default>(
    stream: &mut quinn::RecvStream,
) -> Result<M, PairingError> {
    let mut buf = BytesMut::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        if let Some(msg) = protocol::decode_frame_from_buf::<M>(&mut buf, MAX_FRAME_PAIRING_BYTES)? {
            return Ok(msg);
        }
        match stream.read(&mut chunk).await.map_err(transport::TransportError::from)? {
            Some(n) if n > 0 => {
                buf.extend_from_slice(&chunk[..n]);
            }
            _ => return Err(transport::TransportError::ConnectionClosed.into()),
        }
    }
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Initiator (QR displayer) pairing state.
pub struct InitiatorPairing {
    identity_signer: Arc<dyn IdentitySigner>,
    transport_cert: Arc<TransportCertificate>,
    trust_store: TrustStore,
    replay_cache: Arc<ReplayCache>,
    ephemeral_x25519: Option<EphemeralX25519>,
    session_token: Option<[u8; 16]>,
    qr_payload: Option<QrPayload>,
}

impl InitiatorPairing {
    pub fn new(
        identity_signer: Arc<dyn IdentitySigner>,
        transport_cert: Arc<TransportCertificate>,
        trust_store: TrustStore,
        replay_cache: Arc<ReplayCache>,
    ) -> Self {
        Self {
            identity_signer,
            transport_cert,
            trust_store,
            replay_cache,
            ephemeral_x25519: None,
            session_token: None,
            qr_payload: None,
        }
    }

    /// Generate the QR payload to display to the scanning responder.
    pub fn generate_qr(&mut self, endpoint: String) -> Result<QrPayload, PairingError> {
        let token = SessionToken::new(std::time::Duration::from_secs(MAX_PAIRING_SESSION_SECS));
        let token_bytes = *token.bytes();
        self.replay_cache.register(token_bytes)?;

        let eph = EphemeralX25519::generate();
        let eph_pub = eph.public_key();
        let pubkey = self.identity_signer.verifying_key()?.to_bytes();

        let transcript = build_initiator_transcript(&InitiatorTranscriptInputs {
            qr_format_version: QR_FORMAT_VERSION,
            protocol_version: 1,
            identity_pubkey: &pubkey,
            x25519_ephemeral: eph_pub.as_bytes(),
            session_token: &token_bytes,
            transport_spki_hash: &self.transport_cert.spki_hash,
        });

        let sig = self.identity_signer.sign(&transcript)?;

        let qr = QrPayload {
            format_version: QR_FORMAT_VERSION,
            role: QR_ROLE_INITIATOR,
            identity_pubkey: pubkey,
            x25519_ephemeral: *eph_pub.as_bytes(),
            transport_spki_hash: self.transport_cert.spki_hash,
            session_token: token_bytes,
            signature: sig.to_bytes(),
            endpoint,
        };

        self.ephemeral_x25519 = Some(eph);
        self.session_token = Some(token_bytes);
        self.qr_payload = Some(qr.clone());

        Ok(qr)
    }

    /// Complete pairing over the established QUIC control stream with the responder.
    pub async fn complete_handshake(
        &mut self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        actual_recorded_peer_spki_hash: [u8; 32],
    ) -> Result<TrustedPeer, PairingError> {
        let local_eph = self
            .ephemeral_x25519
            .take()
            .ok_or_else(|| PairingError::InvalidQr("QR not initialized".to_string()))?;
        let expected_token = self
            .session_token
            .ok_or_else(|| PairingError::InvalidQr("Session token missing".to_string()))?;
        let qr = self
            .qr_payload
            .as_ref()
            .ok_or_else(|| PairingError::InvalidQr("QR payload missing".to_string()))?;

        // 1. Receive KeyExchangeResponse from responder
        let resp: KeyExchangeResponse = read_msg(recv_stream).await?;

        // 2. Validate token and consume atomically from replay cache
        if resp.session_token != expected_token {
            return Err(PairingError::SessionTokenMismatch);
        }
        self.replay_cache.consume(&expected_token)?;

        // 3. Verify recorded TLS transport cert SPKI matches claimed SPKI hash in response
        let resp_claimed_spki: [u8; 32] = resp
            .resp_transport_spki_hash
            .as_slice()
            .try_into()
            .map_err(|_| PairingError::SpkiMismatch)?;
        if resp_claimed_spki != actual_recorded_peer_spki_hash {
            return Err(PairingError::SpkiMismatch);
        }

        let resp_pubkey_bytes: [u8; 32] = resp
            .identity_pubkey
            .as_slice()
            .try_into()
            .map_err(|_| PairingError::InvalidQr("Invalid responder public key".to_string()))?;
        let resp_x25519_bytes: [u8; 32] = resp
            .x25519_ephemeral
            .as_slice()
            .try_into()
            .map_err(|_| PairingError::InvalidQr("Invalid responder X25519 key".to_string()))?;

        // 4. Verify responder signature over responder transcript
        let resp_transcript = build_responder_transcript(&ResponderTranscriptInputs {
            qr_format_version: resp.qr_format_version as u8,
            protocol_version: resp.protocol_version,
            identity_pubkey: &resp_pubkey_bytes,
            x25519_ephemeral: &resp_x25519_bytes,
            session_token: &expected_token,
            transport_spki_hash: &resp_claimed_spki,
        });

        let resp_sig = Signature::from_slice(&resp.signature)
            .map_err(|_| PairingError::SignatureInvalid)?;
        let resp_verifying_key = VerifyingKey::from_bytes(&resp_pubkey_bytes)
            .map_err(|_| PairingError::SignatureInvalid)?;
        resp_verifying_key
            .verify_strict(&resp_transcript, &resp_sig)
            .map_err(|_| PairingError::SignatureInvalid)?;

        // 5. Diffie-Hellman & Confirmation Key derivation
        let remote_eph_pub = X25519PublicKey::from(resp_x25519_bytes);
        let dh_output = local_eph.diffie_hellman(&remote_eph_pub);
        let (confirmation_key, _) = derive_pairing_keys(&dh_output, &expected_token)?;

        // 6. Build full transcript
        let init_transcript = build_initiator_transcript(&InitiatorTranscriptInputs {
            qr_format_version: qr.format_version,
            protocol_version: 1,
            identity_pubkey: &qr.identity_pubkey,
            x25519_ephemeral: &qr.x25519_ephemeral,
            session_token: &expected_token,
            transport_spki_hash: &qr.transport_spki_hash,
        });
        let full_transcript = build_full_transcript(&init_transcript, &resp_transcript);

        // 7. Send PairConfirm (initiator MAC)
        let confirm_init_mac = confirm_mac_initiator(&confirmation_key, &full_transcript)?;
        write_msg(
            send_stream,
            &PairConfirm {
                mac: confirm_init_mac.to_vec(),
            },
        )
        .await?;

        // 8. Receive PairConfirm from responder
        let confirm_resp_msg: PairConfirm = read_msg(recv_stream).await?;
        let confirm_resp_mac: [u8; 32] = confirm_resp_msg
            .mac
            .as_slice()
            .try_into()
            .map_err(|_| PairingError::MacMismatch)?;

        let expected_resp_mac = confirm_mac_responder(&confirmation_key, &full_transcript)?;
        verify_confirm_mac(&confirm_resp_mac, &expected_resp_mac)?;

        // 9. ATOMIC persistence to trust store only after both MACs verify
        let fingerprint = Fingerprint::from_pubkey_bytes(&resp_pubkey_bytes).to_string();
        let peer = TrustedPeer {
            fingerprint,
            identity_pubkey: resp_pubkey_bytes,
            transport_spki_hash: resp_claimed_spki,
            display_name: String::new(),
            paired_at: current_unix_timestamp(),
        };

        self.trust_store.add_peer(&peer)?;
        Ok(peer)
    }
}

/// Responder (scanner) pairing state.
pub struct ResponderPairing {
    identity_signer: Arc<dyn IdentitySigner>,
    transport_cert: Arc<TransportCertificate>,
    trust_store: TrustStore,
}

impl ResponderPairing {
    pub fn new(
        identity_signer: Arc<dyn IdentitySigner>,
        transport_cert: Arc<TransportCertificate>,
        trust_store: TrustStore,
    ) -> Self {
        Self {
            identity_signer,
            transport_cert,
            trust_store,
        }
    }

    /// Complete pairing using the scanned QR payload over the established QUIC connection.
    pub async fn complete_handshake(
        &self,
        qr: &QrPayload,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
    ) -> Result<TrustedPeer, PairingError> {
        // 1. Verify initiator signature from QR payload
        qr.verify_signature(1)?;

        // 2. Generate responder ephemeral X25519 keypair
        let local_eph = EphemeralX25519::generate();
        let eph_pub = local_eph.public_key();
        let own_pubkey = self.identity_signer.verifying_key()?.to_bytes();

        // 3. Build responder transcript and sign it
        let resp_transcript = build_responder_transcript(&ResponderTranscriptInputs {
            qr_format_version: qr.format_version,
            protocol_version: 1,
            identity_pubkey: &own_pubkey,
            x25519_ephemeral: eph_pub.as_bytes(),
            session_token: &qr.session_token,
            transport_spki_hash: &self.transport_cert.spki_hash,
        });

        let sig = self.identity_signer.sign(&resp_transcript)?;

        // 4. Send KeyExchangeResponse
        let resp_msg = KeyExchangeResponse {
            qr_format_version: qr.format_version as u32,
            protocol_version: 1,
            identity_pubkey: own_pubkey.to_vec(),
            x25519_ephemeral: eph_pub.as_bytes().to_vec(),
            resp_transport_spki_hash: self.transport_cert.spki_hash.to_vec(),
            session_token: qr.session_token.to_vec(),
            signature: sig.to_bytes().to_vec(),
        };
        write_msg(send_stream, &resp_msg).await?;

        // 5. Diffie-Hellman & Confirmation Key derivation
        let remote_eph_pub = X25519PublicKey::from(qr.x25519_ephemeral);
        let dh_output = local_eph.diffie_hellman(&remote_eph_pub);
        let (confirmation_key, _) = derive_pairing_keys(&dh_output, &qr.session_token)?;

        // 6. Build full transcript
        let init_transcript = build_initiator_transcript(&InitiatorTranscriptInputs {
            qr_format_version: qr.format_version,
            protocol_version: 1,
            identity_pubkey: &qr.identity_pubkey,
            x25519_ephemeral: &qr.x25519_ephemeral,
            session_token: &qr.session_token,
            transport_spki_hash: &qr.transport_spki_hash,
        });
        let full_transcript = build_full_transcript(&init_transcript, &resp_transcript);

        // 7. Receive PairConfirm from initiator
        let confirm_init_msg: PairConfirm = read_msg(recv_stream).await?;
        let confirm_init_mac: [u8; 32] = confirm_init_msg
            .mac
            .as_slice()
            .try_into()
            .map_err(|_| PairingError::MacMismatch)?;

        let expected_init_mac = confirm_mac_initiator(&confirmation_key, &full_transcript)?;
        verify_confirm_mac(&confirm_init_mac, &expected_init_mac)?;

        // 8. Send PairConfirm (responder MAC)
        let confirm_resp_mac = confirm_mac_responder(&confirmation_key, &full_transcript)?;
        write_msg(
            send_stream,
            &PairConfirm {
                mac: confirm_resp_mac.to_vec(),
            },
        )
        .await?;

        // 9. ATOMIC persistence to trust store only after both MACs verify
        let fingerprint = Fingerprint::from_pubkey_bytes(&qr.identity_pubkey).to_string();
        let peer = TrustedPeer {
            fingerprint,
            identity_pubkey: qr.identity_pubkey,
            transport_spki_hash: qr.transport_spki_hash,
            display_name: String::new(),
            paired_at: current_unix_timestamp(),
        };

        self.trust_store.add_peer(&peer)?;
        Ok(peer)
    }
}
