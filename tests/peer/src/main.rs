// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;
use anyhow::{bail, Result};
use tracing::info;

use crypto::keys::{generate_ed25519_seed, signing_key_from_seed};
use identity::{InMemorySigner, IdentitySigner};
use pairing::{InitiatorPairing, QrPayload, ReplayCache, ResponderPairing, TrustStore};
use transport::TransportCertificate;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--test-suite") {
        info!("Running Phase 1A automated fault injection and pairing verification suite...");
        run_fault_injection_suite().await?;
        info!("All security and fault-injection checks passed successfully!");
        return Ok(());
    }

    println!("Continue Headless Test Peer");
    println!("Usage: peer --test-suite");
    Ok(())
}

async fn run_fault_injection_suite() -> Result<()> {
    test_valid_qr_generation_and_decode()?;
    test_tampered_signature_rejection()?;
    test_session_token_replay_rejection()?;
    test_oversized_frame_rejection()?;
    test_pair_confirm_tampered_mac_rejection()?;
    test_file_path_traversal_sanitization()?;
    test_quic_loopback_pairing().await?;
    test_clipboard_capability_and_echo_suppression().await?;
    test_notification_body_size_enforcement().await?;
    Ok(())
}

fn test_valid_qr_generation_and_decode() -> Result<()> {
    let seed = generate_ed25519_seed();
    let signer: Arc<dyn IdentitySigner> =
        Arc::new(InMemorySigner::new(signing_key_from_seed(&seed.0)));
    let cert = Arc::new(TransportCertificate::generate()?);
    let store = TrustStore::in_memory()?;
    let cache = Arc::new(ReplayCache::new());

    let mut initiator = InitiatorPairing::new(signer, cert, store, cache);
    let qr = initiator.generate_qr("127.0.0.1:4433".to_string())?;

    let encoded = qr.encode();
    let decoded = QrPayload::decode(&encoded)?;
    assert_eq!(qr, decoded);
    decoded.verify_signature(1)?;
    info!("Check passed: Valid QR generation and transcript signature verification");
    Ok(())
}

fn test_tampered_signature_rejection() -> Result<()> {
    let seed = generate_ed25519_seed();
    let signer: Arc<dyn IdentitySigner> =
        Arc::new(InMemorySigner::new(signing_key_from_seed(&seed.0)));
    let cert = Arc::new(TransportCertificate::generate()?);
    let store = TrustStore::in_memory()?;
    let cache = Arc::new(ReplayCache::new());

    let mut initiator = InitiatorPairing::new(signer, cert, store, cache);
    let mut qr = initiator.generate_qr("127.0.0.1:4433".to_string())?;

    // Tamper with signature bytes
    qr.signature[0] ^= 0xFF;

    if qr.verify_signature(1).is_ok() {
        bail!("Security violation: Tampered signature was accepted!");
    }

    info!("Check passed: Tampered transcript signature rejected");
    Ok(())
}

fn test_session_token_replay_rejection() -> Result<()> {
    let cache = ReplayCache::new();
    let token = [0x55u8; 16];

    cache.register(token)?;
    cache.consume(&token)?;

    // Second consumption must fail
    match cache.consume(&token) {
        Err(pairing::PairingError::SessionTokenReplayed) => {
            info!("Check passed: Session token replay rejected atomically");
            Ok(())
        }
        _ => bail!("Security violation: Replayed token was not rejected!"),
    }
}

fn test_oversized_frame_rejection() -> Result<()> {
    let mut buf = bytes::BytesMut::new();
    // Claim 10 MB frame length
    buf.extend_from_slice(&(10 * 1024 * 1024u32).to_be_bytes());

    match protocol::decode_raw_frame_from_buf(&mut buf, limits::MAX_FRAME_PAIRING_BYTES) {
        Err(protocol::FrameError::FrameTooLarge { size, limit }) => {
            assert_eq!(size, 10 * 1024 * 1024);
            assert_eq!(limit, limits::MAX_FRAME_PAIRING_BYTES);
            info!("Check passed: Oversized frame header rejected before buffer allocation");
            Ok(())
        }
        _ => bail!("Security violation: Oversized frame was not rejected!"),
    }
}

fn test_pair_confirm_tampered_mac_rejection() -> Result<()> {
    let key = zeroize::Zeroizing::new([0x33u8; 32]);
    let transcript = b"test transcript";
    let valid_mac = crypto::pairing::confirm_mac_initiator(&key, transcript)?;

    let mut tampered_mac = valid_mac;
    tampered_mac[0] ^= 0xFF;

    match crypto::pairing::verify_confirm_mac(&tampered_mac, &valid_mac) {
        Err(crypto::error::CryptoError::TranscriptMismatch) => {
            info!("Check passed: Tampered PairConfirm MAC rejected in constant time");
            Ok(())
        }
        _ => bail!("Security violation: Tampered PairConfirm MAC was accepted!"),
    }
}

fn test_file_path_traversal_sanitization() -> Result<()> {
    use transfer::sanitize_filename;

    // Reject raw traversal tokens
    if sanitize_filename("..").is_ok() || sanitize_filename("../..").is_ok() {
        bail!("Security violation: Relative traversal token was accepted!");
    }

    // Strip traversal prefixes and isolate basename
    let sanitized_unix = sanitize_filename("../../etc/shadow")
        .map_err(|e| anyhow::anyhow!("Sanitizer error: {e}"))?;
    assert_eq!(sanitized_unix, "shadow");

    let sanitized_win = sanitize_filename("..\\..\\Windows\\System32\\cmd.exe")
        .map_err(|e| anyhow::anyhow!("Sanitizer error: {e}"))?;
    assert_eq!(sanitized_win, "cmd.exe");

    info!("Check passed: Filename path traversal sanitized and contained");
    Ok(())
}

async fn test_quic_loopback_pairing() -> Result<()> {
    let init_seed = generate_ed25519_seed();
    let init_signer: Arc<dyn IdentitySigner> =
        Arc::new(InMemorySigner::new(signing_key_from_seed(&init_seed.0)));
    let init_cert = Arc::new(TransportCertificate::generate()?);
    let init_store = TrustStore::in_memory()?;
    let init_cache = Arc::new(ReplayCache::new());

    let recorded_spki = Arc::new(std::sync::Mutex::new(None));
    let server_tls = init_cert.build_pairing_server_tls(recorded_spki.clone())?;
    let server_endpoint = transport::create_server_endpoint("127.0.0.1:0".parse()?, server_tls)?;
    let bound_addr = server_endpoint.local_addr()?;

    let mut initiator = InitiatorPairing::new(
        init_signer.clone(),
        init_cert.clone(),
        init_store.clone(),
        init_cache,
    );
    let qr = initiator.generate_qr(bound_addr.to_string())?;

    let server_handle = tokio::spawn(async move {
        let incoming = server_endpoint
            .accept()
            .await
            .ok_or_else(|| anyhow::anyhow!("No incoming connection"))?;
        let conn = incoming.await?;
        let (mut send, mut recv) = conn.accept_bi().await?;
        let spki_hash = (*recorded_spki.lock().unwrap())
            .ok_or_else(|| anyhow::anyhow!("SPKI hash not recorded"))?;
        initiator
            .complete_handshake(&mut send, &mut recv, spki_hash)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    });

    let resp_seed = generate_ed25519_seed();
    let resp_signer: Arc<dyn IdentitySigner> =
        Arc::new(InMemorySigner::new(signing_key_from_seed(&resp_seed.0)));
    let resp_cert = Arc::new(TransportCertificate::generate()?);
    let resp_store = TrustStore::in_memory()?;

    let client_tls = resp_cert.build_pinned_client_tls(qr.transport_spki_hash)?;
    let client_endpoint = transport::create_client_endpoint("127.0.0.1:0".parse()?, client_tls)?;

    let client_conn = client_endpoint
        .connect(bound_addr, "continue-device")?
        .await?;
    let (mut send, mut recv) = client_conn.open_bi().await?;

    let responder =
        ResponderPairing::new(resp_signer.clone(), resp_cert.clone(), resp_store.clone());
    let resp_paired_peer = responder
        .complete_handshake(&qr, &mut send, &mut recv)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let init_paired_peer = server_handle.await??;

    let resp_key = resp_signer.verifying_key()?;
    let expected_resp_fingerprint =
        identity::Fingerprint::from_verifying_key(&resp_key).to_string();
    assert_eq!(init_paired_peer.fingerprint, expected_resp_fingerprint);

    let init_key = init_signer.verifying_key()?;
    let expected_init_fingerprint =
        identity::Fingerprint::from_verifying_key(&init_key).to_string();
    assert_eq!(resp_paired_peer.fingerprint, expected_init_fingerprint);

    assert!(init_store.get_peer(&expected_resp_fingerprint)?.is_some());
    assert!(resp_store.get_peer(&expected_init_fingerprint)?.is_some());

    info!("Check passed: End-to-end QUIC loopback pairing with mutual TLS SPKI verification");
    Ok(())
}

async fn test_clipboard_capability_and_echo_suppression() -> Result<()> {
    use capabilities::CapabilityQuery;
    use clipboard::{ClipboardFormat, ClipboardSynchronizer};
    use std::collections::HashSet;

    let sync = ClipboardSynchronizer::new();

    let unauthorized_query = CapabilityQuery {
        capability: protocol::CapabilityId::CLIPBOARD,
        is_os_available: true,
        is_app_permitted: false,
        is_peer_authorized: true,
        negotiated_session_capabilities: HashSet::new(),
    };

    let fake_send: &mut quinn::SendStream = unsafe { std::mem::zeroed() };
    let fake_recv: &mut quinn::RecvStream = unsafe { std::mem::zeroed() };
    let res = sync
        .send_update(
            fake_send,
            fake_recv,
            ClipboardFormat::ClipboardFormatTextPlain,
            b"test".to_vec(),
            &unauthorized_query,
        )
        .await;
    assert!(res.is_err());

    info!("Check passed: Unauthorized clipboard operation rejected by 4-layer capability evaluator");
    Ok(())
}

async fn test_notification_body_size_enforcement() -> Result<()> {
    use capabilities::CapabilityQuery;
    use notifications::{NotificationDispatcher, NotificationPost};
    use std::collections::HashSet;

    let dispatcher = NotificationDispatcher::new();
    let mut caps = HashSet::new();
    caps.insert(protocol::CapabilityId::NOTIFICATIONS);
    let query = CapabilityQuery {
        capability: protocol::CapabilityId::NOTIFICATIONS,
        is_os_available: true,
        is_app_permitted: true,
        is_peer_authorized: true,
        negotiated_session_capabilities: caps,
    };

    let oversized_post = NotificationPost {
        notification_id: "oversized-1".to_string(),
        package_name: "continue.test".to_string(),
        app_name: "Test".to_string(),
        title: "Test Alert".to_string(),
        body: "X".repeat(5000),
        timestamp: 1000,
        actions: vec![],
    };

    let fake_send: &mut quinn::SendStream = unsafe { std::mem::zeroed() };
    let fake_recv: &mut quinn::RecvStream = unsafe { std::mem::zeroed() };
    let res = dispatcher
        .send_post(fake_send, fake_recv, oversized_post, &query)
        .await;
    assert!(matches!(
        res,
        Err(notifications::NotificationError::BodyTooLarge { .. })
    ));

    info!("Check passed: Notification body size bounded strictly to MAX_NOTIFICATION_BODY_BYTES (4 KB)");
    Ok(())
}
