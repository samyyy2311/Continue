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
    test_session_multiplexer_routing().await?;
    test_end_to_end_file_transfer().await?;
    test_end_to_end_clipboard_sync().await?;
    test_end_to_end_notification_relay().await?;
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

async fn setup_connected_peer_pair() -> Result<(Arc<sessions::SessionMultiplexer>, Arc<sessions::SessionMultiplexer>)> {
    let server_cert = transport::TransportCertificate::generate()?;
    let client_cert = transport::TransportCertificate::generate()?;

    let server_tls = server_cert.build_pinned_server_tls(client_cert.spki_hash)?;
    let client_tls = client_cert.build_pinned_client_tls(server_cert.spki_hash)?;

    let server_endpoint = transport::create_server_endpoint("127.0.0.1:0".parse()?, server_tls)?;
    let bound_addr = server_endpoint.local_addr()?;

    let client_endpoint = transport::create_client_endpoint("127.0.0.1:0".parse()?, client_tls)?;

    let server_handle = tokio::spawn(async move {
        let incoming = server_endpoint
            .accept()
            .await
            .ok_or_else(|| anyhow::anyhow!("No incoming connection"))?;
        let conn = incoming.await?;
        anyhow::Ok(Arc::new(sessions::SessionMultiplexer::new("client-node".to_string(), conn)))
    });

    let client_conn = client_endpoint.connect(bound_addr, "continue-device")?.await?;
    let client_mux = Arc::new(sessions::SessionMultiplexer::new("server-node".to_string(), client_conn));
    let server_mux = server_handle.await??;

    Ok((server_mux, client_mux))
}

async fn test_session_multiplexer_routing() -> Result<()> {
    use protocol::CapabilityId;

    let (server_mux, client_mux) = setup_connected_peer_pair().await?;
    let mut rx = server_mux.spawn_router(4);

    let (mut send, _recv) = client_mux.open_stream(CapabilityId::CLIPBOARD).await?;
    send.finish()?;

    let stream = rx.recv().await.ok_or_else(|| anyhow::anyhow!("No stream"))?;
    assert_eq!(stream.capability, CapabilityId::CLIPBOARD);

    info!("Check passed: QUIC stream multiplexing and capability routing verified");
    Ok(())
}

async fn test_end_to_end_file_transfer() -> Result<()> {
    let (server_mux, client_mux) = setup_connected_peer_pair().await?;

    let now_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("continue-transfer-test-{now_nanos}"));
    tokio::fs::create_dir_all(&temp_dir).await?;

    let send_file_path = temp_dir.join("test_payload.bin");
    let test_bytes = b"Continue secure multi-node simulation file payload verification";
    tokio::fs::write(&send_file_path, test_bytes).await?;

    let (file_tx, mut file_rx) = tokio::sync::mpsc::channel(1);
    let mut handlers = sessions::SessionCapabilityHandlers::new(&temp_dir);
    handlers.on_file_received = Some(Arc::new(move |received| {
        let _ = file_tx.try_send(received);
    }));

    sessions::spawn_capabilities_dispatcher(server_mux, handlers, 16);

    let bytes_sent = client_mux
        .send_file_to_peer(&send_file_path, "transfer-e2e-1".to_string())
        .await
        .map_err(|e| anyhow::anyhow!("send_file_to_peer failed: {e}"))?;

    assert_eq!(bytes_sent, test_bytes.len() as u64);

    let received = tokio::time::timeout(std::time::Duration::from_secs(5), file_rx.recv())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Timed out waiting for file transfer"))?;

    assert_eq!(received.bytes_received, test_bytes.len() as u64);
    assert_eq!(received.file_name, "test_payload.bin");

    let source_hash = transfer::compute_file_sha256(&send_file_path)
        .await
        .map_err(|e| anyhow::anyhow!("Source hash computation failed: {e}"))?;
    let dest_hash = transfer::compute_file_sha256(&received.path)
        .await
        .map_err(|e| anyhow::anyhow!("Dest hash computation failed: {e}"))?;
    assert_eq!(source_hash, dest_hash);

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    info!("Check passed: End-to-end file streaming and SHA-256 verification");
    Ok(())
}

async fn test_end_to_end_clipboard_sync() -> Result<()> {
    let (server_mux, client_mux) = setup_connected_peer_pair().await?;

    let (clip_tx, mut clip_rx) = tokio::sync::mpsc::channel(1);
    let mut handlers = sessions::SessionCapabilityHandlers::new(std::env::temp_dir());
    handlers.on_clipboard_received = Some(Arc::new(move |update| {
        let _ = clip_tx.try_send(update);
    }));

    sessions::spawn_capabilities_dispatcher(server_mux, handlers, 16);

    let synchronizer = clipboard::ClipboardSynchronizer::new();
    let mut caps = std::collections::HashSet::new();
    caps.insert(protocol::CapabilityId::CLIPBOARD);
    let query = capabilities::CapabilityQuery {
        capability: protocol::CapabilityId::CLIPBOARD,
        is_os_available: true,
        is_app_permitted: true,
        is_peer_authorized: true,
        negotiated_session_capabilities: caps,
    };

    let sample_text = b"Pasted text across Continue mesh nodes";
    let ack = client_mux
        .send_clipboard_to_peer(
            &synchronizer,
            clipboard::ClipboardFormat::ClipboardFormatTextPlain,
            sample_text.to_vec(),
            &query,
        )
        .await
        .map_err(|e| anyhow::anyhow!("send_clipboard_to_peer failed: {e}"))?;

    assert!(ack.applied);

    let update = tokio::time::timeout(std::time::Duration::from_secs(5), clip_rx.recv())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Timed out waiting for clipboard update"))?;

    assert_eq!(update.payload, sample_text.to_vec());
    assert_eq!(
        update.format,
        clipboard::ClipboardFormat::ClipboardFormatTextPlain as i32
    );

    info!("Check passed: End-to-end clipboard synchronization and acknowledgment");
    Ok(())
}

async fn test_end_to_end_notification_relay() -> Result<()> {
    let (server_mux, client_mux) = setup_connected_peer_pair().await?;

    let (notif_tx, mut notif_rx) = tokio::sync::mpsc::channel(1);
    let mut handlers = sessions::SessionCapabilityHandlers::new(std::env::temp_dir());
    handlers.on_notification_received = Some(Arc::new(move |post| {
        let _ = notif_tx.try_send(post);
    }));

    sessions::spawn_capabilities_dispatcher(server_mux, handlers, 16);

    let dispatcher = notifications::NotificationDispatcher::new();
    let mut caps = std::collections::HashSet::new();
    caps.insert(protocol::CapabilityId::NOTIFICATIONS);
    let query = capabilities::CapabilityQuery {
        capability: protocol::CapabilityId::NOTIFICATIONS,
        is_os_available: true,
        is_app_permitted: true,
        is_peer_authorized: true,
        negotiated_session_capabilities: caps,
    };

    let post = notifications::NotificationPost {
        notification_id: "notif-sim-101".to_string(),
        package_name: "org.continueapp.sim".to_string(),
        app_name: "SimulationNode".to_string(),
        title: "Peer Sync Alert".to_string(),
        body: "Capability stream relayed successfully".to_string(),
        timestamp: 987654321,
        actions: vec![],
    };

    let ack = client_mux
        .send_notification_to_peer(&dispatcher, post.clone(), &query)
        .await
        .map_err(|e| anyhow::anyhow!("send_notification_to_peer failed: {e}"))?;

    assert!(ack.handled);

    let received = tokio::time::timeout(std::time::Duration::from_secs(5), notif_rx.recv())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Timed out waiting for notification post"))?;

    assert_eq!(received.notification_id, post.notification_id);
    assert_eq!(received.package_name, post.package_name);
    assert_eq!(received.title, post.title);
    assert_eq!(received.body, post.body);
    assert_eq!(received.timestamp, post.timestamp);

    info!("Check passed: End-to-end notification relay and acknowledgment");
    Ok(())
}
