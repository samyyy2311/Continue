// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use capabilities::CapabilityQuery;
use clipboard::ClipboardFormat;
use pairing::{TrustStore, TrustedPeer};
use protocol::CapabilityId;
use sessions::{
    accept_peers, connect_paired_peers, listen_for_peers, Direction, ReconnectPolicy,
    RegistryConfig, SessionCapabilityHandlers, SessionRegistry, SessionState,
};
use tokio::net::UdpSocket;
use transport::{
    connect_pinned, create_server_endpoint, extract_peer_spki_hash, DialConfig,
    TransportCertificate, TransportError,
};

const LOW: &str = "aaaa-low-fingerprint";
const HIGH: &str = "zzzz-high-fingerprint";

fn fast_config() -> RegistryConfig {
    let mut transport = quinn::TransportConfig::default();
    transport.max_idle_timeout(Some(Duration::from_millis(800).try_into().unwrap()));
    transport.keep_alive_interval(Some(Duration::from_millis(200)));
    RegistryConfig {
        dial: DialConfig {
            transport: Arc::new(transport),
            timeout: Duration::from_millis(500),
        },
        reconnect: ReconnectPolicy::with_limits(
            4,
            Duration::from_millis(100),
            Duration::from_millis(400),
        ),
    }
}

struct Node {
    cert: Arc<TransportCertificate>,
    registry: SessionRegistry,
    listen_addr: SocketAddr,
    endpoint: quinn::Endpoint,
    clips: Arc<Mutex<Vec<Vec<u8>>>>,
}

/// A device that accepts sessions from `trusted` peers and registers them as inbound.
fn node(
    fingerprint: &str,
    cert: TransportCertificate,
    trusted: Vec<(&'static str, [u8; 32])>,
) -> Node {
    let cert = Arc::new(cert);
    let clips = Arc::new(Mutex::new(Vec::new()));
    let mut handlers = SessionCapabilityHandlers::new(std::env::temp_dir());
    let received = clips.clone();
    handlers.on_clipboard_received = Some(Arc::new(move |_peer, update| {
        received.lock().unwrap().push(update.payload);
    }));
    let registry = SessionRegistry::new(
        fingerprint.to_string(),
        cert.clone(),
        handlers,
        fast_config(),
        None,
    );

    let allowed: HashSet<[u8; 32]> = trusted.iter().map(|(_, spki)| *spki).collect();
    let server_tls = cert
        .build_trusted_peers_server_tls(Arc::new(RwLock::new(allowed)))
        .unwrap();
    let endpoint = create_server_endpoint("127.0.0.1:0".parse().unwrap(), server_tls).unwrap();
    let listen_addr = endpoint.local_addr().unwrap();

    let accept = endpoint.clone();
    let inbound = registry.clone();
    tokio::spawn(async move {
        while let Some(incoming) = accept.accept().await {
            let registry = inbound.clone();
            let trusted = trusted.clone();
            tokio::spawn(async move {
                let Ok(conn) = incoming.await else { return };
                let Some(spki) = extract_peer_spki_hash(&conn) else {
                    return;
                };
                if let Some((peer, _)) = trusted.iter().find(|(_, s)| *s == spki) {
                    registry.attach(peer, spki, conn, Direction::Inbound);
                }
            });
        }
    });

    Node {
        cert,
        registry,
        listen_addr,
        endpoint,
        clips,
    }
}

/// Forwards UDP between one client and a switchable target, and can black-hole traffic
/// to imitate a dropped network.
struct Relay {
    addr: SocketAddr,
    target: Arc<Mutex<SocketAddr>>,
    dropping: Arc<AtomicBool>,
}

async fn relay(target: SocketAddr) -> Relay {
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let addr = socket.local_addr().unwrap();
    let target = Arc::new(Mutex::new(target));
    let dropping = Arc::new(AtomicBool::new(false));

    let (forward_to, drop_flag) = (target.clone(), dropping.clone());
    tokio::spawn(async move {
        let mut client: Option<SocketAddr> = None;
        let mut buf = vec![0u8; 65536];
        while let Ok((n, from)) = socket.recv_from(&mut buf).await {
            if drop_flag.load(Ordering::Relaxed) {
                continue;
            }
            let server = *forward_to.lock().unwrap();
            let to = if from == server {
                match client {
                    Some(client) => client,
                    None => continue,
                }
            } else {
                client = Some(from);
                server
            };
            let _ = socket.send_to(&buf[..n], to).await;
        }
    });

    Relay {
        addr,
        target,
        dropping,
    }
}

async fn eventually(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(8);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for: {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn holds(what: &str, duration: Duration, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        assert!(condition(), "stopped holding: {what}");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn send_clip(from: &SessionRegistry, to: &str, text: &str) {
    let mux = from.get(to).expect("connected");
    let query = CapabilityQuery {
        capability: CapabilityId::CLIPBOARD,
        is_os_available: true,
        is_app_permitted: true,
        is_peer_authorized: true,
        negotiated_session_capabilities: HashSet::from([CapabilityId::CLIPBOARD]),
    };
    mux.send_clipboard_to_peer(ClipboardFormat::TextPlain, text.as_bytes().to_vec(), &query)
        .await
        .expect("clipboard delivered");
}

fn pair() -> (Node, Node) {
    let low_cert = TransportCertificate::generate().unwrap();
    let high_cert = TransportCertificate::generate().unwrap();
    let (low_spki, high_spki) = (low_cert.spki_hash, high_cert.spki_hash);
    (
        node(LOW, low_cert, vec![(HIGH, high_spki)]),
        node(HIGH, high_cert, vec![(LOW, low_spki)]),
    )
}

#[tokio::test]
async fn connect_to_dead_address_times_out() {
    let (low, _high) = pair();
    let silent = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let dead = silent.local_addr().unwrap();

    let started = Instant::now();
    let result = low.registry.connect(HIGH, [7u8; 32], dead).await;

    assert!(matches!(result, Err(TransportError::ConnectTimeout(addr)) if addr == dead));
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(low.registry.state(HIGH), SessionState::Disconnected);
    assert!(low.registry.get(HIGH).is_none());
}

#[tokio::test]
async fn lost_connection_is_removed_then_reconnected_and_reauthenticated() {
    let (low, high) = pair();
    let path = relay(high.listen_addr).await;
    let high_spki = high.cert.spki_hash;

    low.registry
        .connect(HIGH, high_spki, path.addr)
        .await
        .unwrap();
    eventually("both sides connected", || {
        low.registry.get(HIGH).is_some() && high.registry.get(LOW).is_some()
    })
    .await;
    let first = low.registry.get(HIGH).unwrap();

    path.dropping.store(true, Ordering::Relaxed);
    eventually("stale session removed on the dialing side", || {
        low.registry.get(HIGH).is_none() && low.registry.state(HIGH) == SessionState::Reconnecting
    })
    .await;
    eventually("stale session removed on the accepting side", || {
        high.registry.get(LOW).is_none()
    })
    .await;
    assert!(first.connection().close_reason().is_some());

    path.dropping.store(false, Ordering::Relaxed);
    eventually("reconnected", || {
        low.registry.state(HIGH) == SessionState::Connected && high.registry.get(LOW).is_some()
    })
    .await;
    let second = low.registry.get(HIGH).unwrap();
    assert!(!Arc::ptr_eq(&first, &second));

    send_clip(&low.registry, HIGH, "after reconnect").await;
    eventually("capability streams work on the new session", || {
        high.clips.lock().unwrap().as_slice() == [b"after reconnect".to_vec()]
    })
    .await;
}

#[tokio::test]
async fn restarted_peer_with_same_identity_is_reconnected() {
    let low_cert = TransportCertificate::generate().unwrap();
    let high_cert = TransportCertificate::generate().unwrap();
    let high_pem = high_cert.pkcs8_der.clone();
    let (low_spki, high_spki) = (low_cert.spki_hash, high_cert.spki_hash);
    let low = node(LOW, low_cert, vec![(HIGH, high_spki)]);
    let high = node(HIGH, high_cert, vec![(LOW, low_spki)]);
    let path = relay(high.listen_addr).await;

    low.registry
        .connect(HIGH, high_spki, path.addr)
        .await
        .unwrap();
    eventually("accepted", || high.registry.get(LOW).is_some()).await;
    send_clip(&high.registry, LOW, "A").await;
    send_clip(&high.registry, LOW, "B").await;
    eventually("consecutive clips accepted in order", || {
        *low.clips.lock().unwrap() == [b"A".to_vec(), b"B".to_vec()]
    })
    .await;

    high.endpoint.close(0u32.into(), b"");
    let restarted = node(
        HIGH,
        TransportCertificate::from_pem_bytes(&high_pem).unwrap(),
        vec![(LOW, low_spki)],
    );
    *path.target.lock().unwrap() = restarted.listen_addr;

    eventually("reconnected to the restarted peer", || {
        low.registry.get(HIGH).is_some() && restarted.registry.get(LOW).is_some()
    })
    .await;

    // The new connection numbers its clipboard updates from one again.
    send_clip(&restarted.registry, LOW, "C").await;
    send_clip(&restarted.registry, LOW, "D").await;
    eventually("the new connection's clips are accepted in order", || {
        *low.clips.lock().unwrap() == [b"A".to_vec(), b"B".to_vec(), b"C".to_vec(), b"D".to_vec()]
    })
    .await;
}

#[tokio::test]
async fn reconnect_refuses_a_peer_with_a_different_key() {
    let (low, high) = pair();
    let path = relay(high.listen_addr).await;
    low.registry
        .connect(HIGH, high.cert.spki_hash, path.addr)
        .await
        .unwrap();

    let impostor = node(
        HIGH,
        TransportCertificate::generate().unwrap(),
        vec![(LOW, low.cert.spki_hash)],
    );
    high.endpoint.close(0u32.into(), b"");
    *path.target.lock().unwrap() = impostor.listen_addr;

    eventually("gave up after the retry policy", || {
        low.registry.state(HIGH) == SessionState::Disconnected
    })
    .await;
    assert!(low.registry.get(HIGH).is_none());
    assert!(impostor.registry.get(LOW).is_none());
}

#[tokio::test]
async fn reconnect_stops_after_retry_policy() {
    let (low, high) = pair();
    let path = relay(high.listen_addr).await;
    low.registry
        .connect(HIGH, high.cert.spki_hash, path.addr)
        .await
        .unwrap();

    path.dropping.store(true, Ordering::Relaxed);
    eventually("started reconnecting", || {
        low.registry.state(HIGH) == SessionState::Reconnecting
    })
    .await;
    eventually("gave up", || {
        low.registry.state(HIGH) == SessionState::Disconnected
    })
    .await;

    path.dropping.store(false, Ordering::Relaxed);
    holds(
        "stays disconnected once retries are spent",
        Duration::from_millis(1500),
        || low.registry.get(HIGH).is_none() && high.registry.get(LOW).is_none(),
    )
    .await;
}

#[tokio::test]
async fn manual_disconnect_does_not_reconnect() {
    let (low, high) = pair();
    low.registry
        .connect(HIGH, high.cert.spki_hash, high.listen_addr)
        .await
        .unwrap();
    eventually("accepted", || high.registry.get(LOW).is_some()).await;

    low.registry.disconnect(HIGH).await;
    assert_eq!(low.registry.state(HIGH), SessionState::Closed);
    eventually("peer saw the disconnect", || {
        high.registry.state(LOW) == SessionState::Disconnected
    })
    .await;

    holds("no reconnect", Duration::from_millis(1500), || {
        low.registry.state(HIGH) == SessionState::Closed && high.registry.get(LOW).is_none()
    })
    .await;
}

#[tokio::test]
async fn peer_disconnect_does_not_reconnect() {
    let (low, high) = pair();
    low.registry
        .connect(HIGH, high.cert.spki_hash, high.listen_addr)
        .await
        .unwrap();
    eventually("accepted", || high.registry.get(LOW).is_some()).await;

    high.registry.disconnect(LOW).await;
    eventually("dialer saw the disconnect", || {
        low.registry.state(HIGH) == SessionState::Disconnected
    })
    .await;

    holds(
        "dialer does not reconnect",
        Duration::from_millis(1500),
        || {
            low.registry.state(HIGH) == SessionState::Disconnected
                && low.registry.get(HIGH).is_none()
        },
    )
    .await;
}

#[tokio::test]
async fn discovery_connects_only_to_the_peer_whose_key_matches() {
    let (low, high) = pair();
    let stranger = node(
        "stranger",
        TransportCertificate::generate().unwrap(),
        vec![],
    );

    let reached = low
        .registry
        .connect_discovered(
            HIGH,
            high.cert.spki_hash,
            &[stranger.listen_addr, high.listen_addr],
            || true,
        )
        .await;

    assert_eq!(reached, Some(high.listen_addr));
    assert_eq!(low.registry.state(HIGH), SessionState::Connected);
    eventually("accepted", || high.registry.get(LOW).is_some()).await;
    assert!(stranger.registry.get(LOW).is_none());
}

#[tokio::test]
async fn discovery_drops_a_device_forgotten_during_the_handshake() {
    let (low, high) = pair();
    // Trusted when the dial starts, forgotten by the time the handshake finishes.
    let checks = std::sync::atomic::AtomicUsize::new(0);
    let still_trusted = || checks.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0;

    let reached = low
        .registry
        .connect_discovered(
            HIGH,
            high.cert.spki_hash,
            &[high.listen_addr],
            still_trusted,
        )
        .await;

    assert_eq!(reached, None);
    assert!(low.registry.get(HIGH).is_none());
}

#[tokio::test]
async fn discovery_leaves_deliberate_disconnects_alone() {
    let (low, high) = pair();
    low.registry
        .connect(HIGH, high.cert.spki_hash, high.listen_addr)
        .await
        .unwrap();
    eventually("accepted", || high.registry.get(LOW).is_some()).await;

    high.registry.disconnect(LOW).await;
    eventually("dialer saw the disconnect", || {
        low.registry.get(HIGH).is_none()
    })
    .await;

    let from_low = low
        .registry
        .connect_discovered(HIGH, high.cert.spki_hash, &[high.listen_addr], || true)
        .await;
    let from_high = high
        .registry
        .connect_discovered(LOW, low.cert.spki_hash, &[low.listen_addr], || true)
        .await;
    assert_eq!((from_low, from_high), (None, None));
    assert!(low.registry.get(HIGH).is_none() && high.registry.get(LOW).is_none());

    low.registry.resume_auto_connect(HIGH);
    let resumed = low
        .registry
        .connect_discovered(HIGH, high.cert.spki_hash, &[high.listen_addr], || true)
        .await;
    assert_eq!(resumed, Some(high.listen_addr));
}

#[tokio::test]
async fn listener_admits_paired_devices_only() {
    let desktop_cert = Arc::new(TransportCertificate::generate().unwrap());
    let phone = node(LOW, TransportCertificate::generate().unwrap(), vec![]);
    let stranger_cert = TransportCertificate::generate().unwrap();

    let trust_store = TrustStore::in_memory().unwrap();
    trust_store
        .add_peer(&TrustedPeer {
            fingerprint: LOW.to_string(),
            identity_pubkey: [1u8; 32],
            transport_spki_hash: phone.cert.spki_hash,
            display_name: "Phone".to_string(),
            paired_at: 1,
        })
        .unwrap();
    // The stranger passes the TLS check but isn't in the trust store, as if it was
    // unpaired while connecting.
    let trusted_keys = Arc::new(RwLock::new(HashSet::from([
        phone.cert.spki_hash,
        stranger_cert.spki_hash,
    ])));

    let desktop = SessionRegistry::new(
        HIGH.to_string(),
        desktop_cert.clone(),
        SessionCapabilityHandlers::new(std::env::temp_dir()),
        fast_config(),
        None,
    );
    let endpoint = listen_for_peers(&desktop_cert, trusted_keys).unwrap();
    let addr = SocketAddr::from(([127, 0, 0, 1], endpoint.local_addr().unwrap().port()));
    tokio::spawn(accept_peers(endpoint, desktop.clone(), trust_store));

    phone
        .registry
        .connect(HIGH, desktop_cert.spki_hash, addr)
        .await
        .unwrap();
    eventually("desktop adopted the phone's connection", || {
        desktop.get(LOW).is_some()
    })
    .await;

    let stranger = connect_pinned(
        &stranger_cert,
        desktop_cert.spki_hash,
        addr,
        &DialConfig::default(),
    )
    .await
    .unwrap();
    let closed = tokio::time::timeout(Duration::from_secs(5), stranger.closed())
        .await
        .expect("stranger was disconnected");
    assert!(
        matches!(closed, quinn::ConnectionError::ApplicationClosed(ref close) if &close.reason[..] == b"untrusted_peer")
    );
}

#[tokio::test]
async fn paired_peer_is_dialed_at_its_saved_address_right_after_pairing() {
    let (low, high) = pair();
    let trust_store = TrustStore::in_memory().unwrap();
    trust_store
        .add_peer(&TrustedPeer {
            fingerprint: HIGH.to_string(),
            identity_pubkey: [1u8; 32],
            transport_spki_hash: high.cert.spki_hash,
            display_name: "Desktop".to_string(),
            paired_at: 1,
        })
        .unwrap();
    let dialer = tokio::spawn(connect_paired_peers(
        low.registry.clone(),
        trust_store.clone(),
    ));
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Saved after the loop's first round, as after pairing; waking it avoids the 10 s wait.
    trust_store
        .set_last_endpoint(HIGH, &high.listen_addr.to_string())
        .unwrap();
    low.registry.redial_now();
    eventually("connected without discovery", || {
        low.registry.state(HIGH) == SessionState::Connected && high.registry.get(LOW).is_some()
    })
    .await;
    dialer.abort();
}

#[tokio::test]
async fn simultaneous_dials_leave_exactly_one_session() {
    let (low, high) = pair();
    let (a, b) = tokio::join!(
        low.registry
            .connect(HIGH, high.cert.spki_hash, high.listen_addr),
        high.registry
            .connect(LOW, low.cert.spki_hash, low.listen_addr),
    );
    a.unwrap();
    b.unwrap();

    // The survivor is the connection dialled by the lower fingerprint.
    let settled = || {
        let (Some(on_low), Some(on_high)) = (low.registry.get(HIGH), high.registry.get(LOW)) else {
            return false;
        };
        on_low.connection().remote_address() == high.listen_addr
            && on_high.connection().remote_address() != low.listen_addr
    };
    eventually("one shared connection", settled).await;
    holds(
        "stays settled without flapping",
        Duration::from_millis(1500),
        || {
            settled()
                && low.registry.state(HIGH) == SessionState::Connected
                && high.registry.state(LOW) == SessionState::Connected
        },
    )
    .await;

    send_clip(&high.registry, LOW, "one session").await;
    eventually("usable in both directions", || {
        low.clips.lock().unwrap().as_slice() == [b"one session".to_vec()]
    })
    .await;
}

#[tokio::test]
async fn closing_a_replaced_connection_keeps_its_replacement() {
    let (low, high) = pair();
    let dial = DialConfig::default();

    let first = transport::connect_pinned(&low.cert, high.cert.spki_hash, high.listen_addr, &dial)
        .await
        .unwrap();
    eventually("first attached", || high.registry.get(LOW).is_some()).await;
    let first_on_high = high.registry.get(LOW).unwrap();

    let second = transport::connect_pinned(&low.cert, high.cert.spki_hash, high.listen_addr, &dial)
        .await
        .unwrap();
    eventually("second replaced the first", || {
        high.registry
            .get(LOW)
            .is_some_and(|current| !Arc::ptr_eq(&current, &first_on_high))
    })
    .await;
    let replacement = high.registry.get(LOW).unwrap();

    first.close(
        sessions::close_code(protocol::v1::DisconnectReason::Error),
        b"",
    );
    holds("replacement survives", Duration::from_millis(1000), || {
        high.registry
            .get(LOW)
            .is_some_and(|current| Arc::ptr_eq(&current, &replacement))
            && high.registry.state(LOW) == SessionState::Connected
    })
    .await;
    drop(second);
}
