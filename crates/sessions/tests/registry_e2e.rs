// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use capabilities::CapabilityQuery;
use clipboard::ClipboardFormat;
use pairing::{TrustStore, TrustedPeer};
use protocol::v1::{
    call, call_action, calls_message, camera_control, computer_action, media_command,
    media_message, messages_message, photos_message, pointer_input, screen_input, touch, Call,
    CallAction, CameraControl, CameraRequest, Contact, Conversation, DeviceLook, DeviceStatus,
    ListContacts, ListConversations, ListPhotos, MediaCommand, MessagesChanged, NowPlaying, Photo,
    PointerInput, PointerMove, PointerStart, ReadConversation, ScreenInput, ScreenRequest,
    SearchResult, SendPhoto, SendText, Snippet, TextMessage, Touch, VideoFrame, VideoStart,
};
use protocol::CapabilityId;
use sessions::{
    accept_peers, connect_paired_peers, listen_for_peers, CallControl, ComputerActions, Direction,
    IncomingEvent, IncomingFiles, MediaControl, MessageStore, PhoneSearch, PhotoLibrary,
    PointerLeave, PointerTarget, ReconnectPolicy, RegistryConfig, Ringer,
    SessionCapabilityHandlers, SessionRegistry, SessionState, SnippetStore, VideoFeed, VideoSource,
    MAX_PHOTOS,
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
    /// Files this device received.
    files: Arc<Mutex<Vec<transfer::ReceivedFile>>>,
    /// Bytes received so far, each time a file coming in reported progress.
    progress: Arc<Mutex<Vec<u64>>>,
    /// Battery readings the peer reported.
    statuses: Arc<Mutex<Vec<DeviceStatus>>>,
    /// How the peer said it looks.
    looks: Arc<Mutex<Vec<DeviceLook>>>,
    /// Photos the peer said it just took.
    taken: Arc<Mutex<Vec<Photo>>>,
    messages: Arc<OneConversation>,
    phone: Arc<RingingPhone>,
    screen: Arc<FakeVideo<ScreenInput>>,
    camera: Arc<FakeVideo<CameraControl>>,
    player: Arc<Player>,
    bell: Arc<Bell>,
    pad: Arc<Pad>,
    desk: Arc<Desk>,
    pinned: Arc<Pinned>,
    now_playing: Arc<Mutex<Vec<NowPlaying>>>,
    /// Calls the peer said were ringing, answered or over.
    calls: Arc<Mutex<Vec<Call>>>,
    /// How many times the peer said its messages changed.
    changes: Arc<Mutex<u32>>,
}

/// One photo, kept at `path`.
struct OnePhoto {
    path: PathBuf,
}

impl PhotoLibrary for OnePhoto {
    fn recent(&self, limit: u32) -> Option<Vec<Photo>> {
        assert!(limit <= MAX_PHOTOS);
        Some(vec![Photo {
            id: "1".to_string(),
            name: "beach.jpg".to_string(),
            taken_at: 1,
            thumbnail: vec![0xff, 0xd8],
        }])
    }

    fn file(&self, id: &str) -> Option<PathBuf> {
        (id == "1").then(|| self.path.clone())
    }
}

/// One conversation, remembering what was sent.
#[derive(Default)]
struct OneConversation {
    sent: Mutex<Vec<(String, String)>>,
}

impl MessageStore for OneConversation {
    fn contacts(&self, _limit: u32) -> Option<Vec<Contact>> {
        Some(vec![Contact {
            name: "Asha".to_string(),
            number: "+15550100".to_string(),
            favorite: true,
            photo: Vec::new(),
        }])
    }

    fn conversations(&self, _limit: u32) -> Option<Vec<Conversation>> {
        Some(vec![Conversation {
            id: "7".to_string(),
            address: "+15550100".to_string(),
            name: "Asha".to_string(),
            snippet: "See you at 6".to_string(),
            at: 1,
            unread: true,
        }])
    }

    fn conversation(&self, id: &str, _limit: u32) -> Option<Vec<TextMessage>> {
        (id == "7").then(|| {
            vec![TextMessage {
                id: "1".to_string(),
                body: "See you at 6".to_string(),
                at: 1,
                outgoing: false,
            }]
        })
    }

    fn send(&self, address: &str, body: &str) -> bool {
        self.sent
            .lock()
            .unwrap()
            .push((address.to_string(), body.to_string()));
        true
    }
}

/// A phone whose ringing call can be answered once.
#[derive(Default)]
struct RingingPhone {
    answered: AtomicBool,
}

impl CallControl for RingingPhone {
    fn answer(&self) -> bool {
        !self.answered.swap(true, Ordering::Relaxed)
    }

    fn decline(&self) -> bool {
        false
    }

    fn silence(&self) -> bool {
        false
    }

    fn dial(&self, _: &str) -> bool {
        false
    }
}

/// A screen or camera that sends two frames once shared, and remembers how it was steered.
struct FakeVideo<C> {
    feed: VideoFeed,
    controls: Mutex<Vec<C>>,
    stopped: AtomicBool,
}

impl<C> FakeVideo<C> {
    fn new(feed: &VideoFeed) -> Arc<Self> {
        Arc::new(Self {
            feed: feed.clone(),
            controls: Mutex::default(),
            stopped: AtomicBool::new(false),
        })
    }
}

impl<R, C: Send> VideoSource<R, C> for FakeVideo<C> {
    fn start(&self, _request: R) -> Option<VideoStart> {
        for (data, key) in [
            (vec![0, 0, 0, 1, 0x67], true),
            (vec![0, 0, 0, 1, 0x41], false),
        ] {
            assert!(self.feed.push(VideoFrame {
                data,
                key,
                ..Default::default()
            }));
        }
        assert!(self.feed.push_audio(vec![1, 2, 3, 4]));
        Some(VideoStart {
            started: true,
            width: 576,
            height: 1280,
            rotation: 0,
        })
    }

    fn control(&self, control: C) {
        self.controls.lock().unwrap().push(control);
    }

    fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
}

#[derive(Default)]
struct Bell(AtomicBool);

impl Ringer for Bell {
    fn ring(&self, on: bool) -> bool {
        self.0.store(on, Ordering::Relaxed);
        true
    }
}

/// A phone with one text that mentions dinner.
struct OneText;

impl PhoneSearch for OneText {
    fn search(&self, query: &str, _: u32) -> Option<Vec<SearchResult>> {
        let text = SearchResult {
            title: "Sam".to_string(),
            detail: "Dinner at 8?".to_string(),
            ..Default::default()
        };
        Some(
            vec![text]
                .into_iter()
                .filter(|t| t.detail.to_lowercase().contains(query))
                .collect(),
        )
    }
}

/// Keeps the snippets a peer sent.
#[derive(Default)]
struct Pinned(Mutex<Vec<Snippet>>);

impl SnippetStore for Pinned {
    fn merge(&self, _: &str, snippets: Vec<Snippet>) {
        self.0.lock().unwrap().extend(snippets);
    }
}

/// A computer that notes what the phone asked it to do.
#[derive(Default)]
struct Desk(Mutex<Vec<computer_action::Body>>);

impl ComputerActions for Desk {
    fn act(&self, action: computer_action::Body) -> bool {
        self.0.lock().unwrap().push(action);
        true
    }
}

/// A phone's pointer: it goes back to the computer as soon as it moves left.
#[derive(Default)]
struct Pad {
    inputs: Mutex<Vec<PointerInput>>,
    leave: Mutex<Option<PointerLeave>>,
    stopped: AtomicBool,
}

impl PointerTarget for Pad {
    fn start(&self, _: PointerStart, leave: PointerLeave) -> bool {
        *self.leave.lock().unwrap() = Some(leave);
        true
    }

    fn input(&self, input: PointerInput) {
        let left = matches!(
            input.body,
            Some(pointer_input::Body::Move(PointerMove { dx, .. })) if dx < 0.0
        );
        self.inputs.lock().unwrap().push(input);
        if let (true, Some(leave)) = (left, &*self.leave.lock().unwrap()) {
            leave(0.25);
        }
    }

    fn stop(&self) {
        self.stopped.store(true, Ordering::Relaxed);
    }
}

/// Remembers the media commands it was given.
#[derive(Default)]
struct Player {
    commands: Mutex<Vec<media_command::Kind>>,
}

impl MediaControl for Player {
    fn command(&self, command: media_command::Kind) -> bool {
        self.commands.lock().unwrap().push(command);
        true
    }
}

/// A device that accepts sessions from `trusted` peers and registers them as inbound.
fn node(
    fingerprint: &str,
    cert: TransportCertificate,
    trusted: Vec<(&'static str, [u8; 32])>,
) -> Node {
    let cert = Arc::new(cert);
    let clips = Arc::new(Mutex::new(Vec::new()));
    let (files, progress) = (
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(Vec::new())),
    );
    let saved = std::env::temp_dir().join(format!("continue-registry-{}", rand::random::<u64>()));
    let heard = progress.clone();
    let mut handlers = SessionCapabilityHandlers::new(saved).with_incoming(
        IncomingFiles::with_listener(Arc::new(move |event| {
            if let IncomingEvent::Progress(file) = event {
                heard.lock().unwrap().push(file.received);
            }
        })),
    );
    let received = clips.clone();
    handlers.on_clipboard_received = Some(Arc::new(move |_peer, update| {
        received.lock().unwrap().push(update.payload);
    }));
    let statuses = Arc::new(Mutex::new(Vec::new()));
    let reported = statuses.clone();
    handlers.on_device_status = Some(Arc::new(move |_peer, status| {
        reported.lock().unwrap().push(status);
    }));
    let looks = Arc::new(Mutex::new(Vec::new()));
    let seen = looks.clone();
    handlers.on_device_look = Some(Arc::new(move |_peer, look| {
        seen.lock().unwrap().push(look);
    }));
    let arrived = files.clone();
    handlers.on_file_received = Some(Arc::new(move |_peer, file| {
        arrived.lock().unwrap().push(file);
    }));
    let photo = std::env::temp_dir().join(format!("continue-photo-{}.jpg", rand::random::<u64>()));
    std::fs::write(&photo, b"a photo").unwrap();
    handlers.photo_library = Some(Arc::new(OnePhoto { path: photo }));
    let messages = Arc::new(OneConversation::default());
    handlers.message_store = Some(messages.clone());
    let changes = Arc::new(Mutex::new(0));
    let changed = changes.clone();
    handlers.on_messages_changed = Some(Arc::new(move |_peer, ()| {
        *changed.lock().unwrap() += 1;
    }));
    let screen = FakeVideo::new(&handlers.screen_feed);
    handlers.screen_source = Some(screen.clone());
    let camera = FakeVideo::new(&handlers.camera_feed);
    handlers.camera_source = Some(camera.clone());
    let bell = Arc::new(Bell::default());
    handlers.ringer = Some(bell.clone());
    let pad = Arc::new(Pad::default());
    handlers.pointer_target = Some(pad.clone());
    let desk = Arc::new(Desk::default());
    handlers.computer_actions = Some(desk.clone());
    let pinned = Arc::new(Pinned::default());
    handlers.snippet_store = Some(pinned.clone());
    handlers.phone_search = Some(Arc::new(OneText));
    let player = Arc::new(Player::default());
    handlers.media_control = Some(player.clone());
    let now_playing = Arc::new(Mutex::new(Vec::new()));
    let heard = now_playing.clone();
    handlers.on_now_playing = Some(Arc::new(move |_peer, playing| {
        heard.lock().unwrap().push(playing);
    }));
    let phone = Arc::new(RingingPhone::default());
    handlers.call_control = Some(phone.clone());
    let calls = Arc::new(Mutex::new(Vec::new()));
    let rang = calls.clone();
    handlers.on_call = Some(Arc::new(move |_peer, call| {
        rang.lock().unwrap().push(call);
    }));
    let taken = Arc::new(Mutex::new(Vec::new()));
    let shown = taken.clone();
    handlers.on_photo_taken = Some(Arc::new(move |_peer, photo| {
        shown.lock().unwrap().push(photo);
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
        files,
        progress,
        statuses,
        looks,
        taken,
        messages,
        changes,
        phone,
        calls,
        screen,
        camera,
        player,
        now_playing,
        bell,
        pad,
        desk,
        pinned,
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
    let query = CapabilityQuery::negotiated(CapabilityId::CLIPBOARD, true);
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
async fn a_paused_device_drops_its_sessions_and_turns_dials_away_until_resumed() {
    let (low, high) = pair();
    low.registry
        .connect(HIGH, high.cert.spki_hash, high.listen_addr)
        .await
        .unwrap();
    eventually("accepted", || high.registry.get(LOW).is_some()).await;

    high.registry.set_paused(true).await;
    eventually("dialer saw the disconnect", || {
        low.registry.get(HIGH).is_none()
    })
    .await;
    let _ = low
        .registry
        .connect(HIGH, high.cert.spki_hash, high.listen_addr)
        .await;
    holds("dials are turned away", Duration::from_millis(1000), || {
        high.registry.get(LOW).is_none()
    })
    .await;

    high.registry.set_paused(false).await;
    low.registry
        .connect(HIGH, high.cert.spki_hash, high.listen_addr)
        .await
        .unwrap();
    eventually("accepted again", || high.registry.get(LOW).is_some()).await;
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

#[tokio::test]
async fn a_closed_duplicate_is_reconnected_not_taken_as_a_disconnect() {
    let (low, high) = pair();
    low.registry
        .connect(HIGH, high.cert.spki_hash, high.listen_addr)
        .await
        .unwrap();
    eventually("accepted", || high.registry.get(LOW).is_some()).await;
    let before = low.registry.get(HIGH).unwrap();

    // As when the two sides kept different duplicates and this one was the dialer's.
    high.registry.get(LOW).unwrap().connection().close(
        sessions::close_code(protocol::v1::DisconnectReason::Redundant),
        b"duplicate",
    );
    eventually("the dialer reconnects", || {
        low.registry
            .get(HIGH)
            .is_some_and(|now| !Arc::ptr_eq(&now, &before))
            && high.registry.state(LOW) == SessionState::Connected
    })
    .await;
}

#[tokio::test]
async fn a_file_carries_on_where_it_stopped_after_the_connection_drops() {
    let (low, high) = pair();
    let path = relay(high.listen_addr).await;
    low.registry
        .connect(HIGH, high.cert.spki_hash, path.addr)
        .await
        .unwrap();
    eventually("connected", || high.registry.get(LOW).is_some()).await;
    let first = low.registry.get(HIGH).unwrap();

    let size = 16 * 1024 * 1024;
    let bytes: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
    let file = std::env::temp_dir().join(format!("continue-resend-{}.bin", rand::random::<u32>()));
    std::fs::write(&file, &bytes).unwrap();

    // The network goes quiet half way through.
    let (registry, dropping, sending) = (low.registry.clone(), path.dropping.clone(), file.clone());
    let dropped = AtomicBool::new(false);
    let send = tokio::spawn(async move {
        registry
            .send_file(
                HIGH,
                &sending,
                "tx-big".to_string(),
                Some(move |done: u64, total: u64| {
                    if done >= total / 2 && !dropped.swap(true, Ordering::Relaxed) {
                        dropping.store(true, Ordering::Relaxed);
                    }
                }),
            )
            .await
    });
    eventually("the connection is lost", || {
        path.dropping.load(Ordering::Relaxed) && low.registry.get(HIGH).is_none()
    })
    .await;
    eventually("the receiver noticed too", || {
        high.registry.get(LOW).is_none()
    })
    .await;
    path.dropping.store(false, Ordering::Relaxed);

    let sent = tokio::time::timeout(Duration::from_secs(20), send)
        .await
        .expect("the send finishes once reconnected")
        .unwrap();
    assert_eq!(sent.unwrap(), size as u64);
    eventually("the file arrived", || high.files.lock().unwrap().len() == 1).await;
    let arrived = high.files.lock().unwrap()[0].path.clone();
    assert_eq!(std::fs::read(arrived).unwrap(), bytes);

    // Started from nothing once; after the drop it carried on rather than going back to 0.
    let progress = high.progress.lock().unwrap().clone();
    assert!(
        progress.windows(2).all(|pair| pair[1] >= pair[0]),
        "never went backwards: {progress:?}"
    );
    assert_eq!(progress.iter().filter(|&&at| at == 0).count(), 1);
    assert!(
        !Arc::ptr_eq(&low.registry.get(HIGH).unwrap(), &first),
        "the rest went over the new connection"
    );
}

#[tokio::test]
async fn connecting_twice_at_once_makes_one_connection() {
    let (low, high) = pair();
    let (first, second) = tokio::join!(
        low.registry
            .connect(HIGH, high.cert.spki_hash, high.listen_addr),
        low.registry
            .connect(HIGH, high.cert.spki_hash, high.listen_addr),
    );
    first.unwrap();
    second.unwrap();

    let session = low.registry.get(HIGH).expect("connected");
    eventually("accepted", || high.registry.get(LOW).is_some()).await;
    holds(
        "the one connection stays",
        Duration::from_millis(1000),
        || {
            low.registry
                .get(HIGH)
                .is_some_and(|now| Arc::ptr_eq(&now, &session))
                && session.connection().close_reason().is_none()
        },
    )
    .await;
}

#[tokio::test]
async fn battery_status_reaches_the_peer_on_connect_and_when_it_changes() {
    let (low, high) = pair();
    let at = |battery_percent, charging| DeviceStatus {
        battery_percent,
        charging,
        ..Default::default()
    };
    low.registry.report_status(at(80, false));

    low.registry
        .connect(HIGH, high.cert.spki_hash, high.listen_addr)
        .await
        .unwrap();
    eventually("the reading taken before connecting arrives", || {
        high.statuses.lock().unwrap().as_slice() == [at(80, false)]
    })
    .await;

    low.registry.report_status(at(80, false));
    low.registry.report_status(at(81, true));
    eventually("a change arrives", || {
        high.statuses.lock().unwrap().last() == Some(&at(81, true))
    })
    .await;
    assert_eq!(
        high.statuses.lock().unwrap().len(),
        2,
        "an unchanged reading isn't sent again"
    );
}

#[tokio::test]
async fn how_a_device_looks_reaches_the_peer_on_connect_and_when_it_changes() {
    let (low, high) = pair();
    let wallpaper = DeviceLook {
        wallpaper_color: 0x2457d6,
        wallpaper: vec![0xff, 0xd8, 0xff, 0xe0],
    };
    low.registry.report_look(wallpaper.clone());

    low.registry
        .connect(HIGH, high.cert.spki_hash, high.listen_addr)
        .await
        .unwrap();
    eventually("the look set before connecting arrives", || {
        high.looks.lock().unwrap().as_slice() == [wallpaper.clone()]
    })
    .await;

    let changed = DeviceLook {
        wallpaper_color: 0x1e7a45,
        ..wallpaper.clone()
    };
    low.registry.report_look(wallpaper.clone());
    low.registry.report_look(changed.clone());
    eventually("a new wallpaper arrives", || {
        high.looks.lock().unwrap().last() == Some(&changed)
    })
    .await;
    assert_eq!(
        high.looks.lock().unwrap().len(),
        2,
        "an unchanged look isn't sent again"
    );
}

#[tokio::test]
async fn a_computer_lists_the_phones_photos_and_gets_one_sent_over() {
    let (low, high) = pair();
    low.registry
        .connect(HIGH, high.cert.spki_hash, high.listen_addr)
        .await
        .unwrap();
    let mux = low.registry.get(HIGH).unwrap();

    let listed = mux
        .send_photos_message(photos_message::Body::List(ListPhotos { limit: 1000 }))
        .await
        .unwrap();
    assert!(listed.available);
    assert_eq!(listed.photos.len(), 1);

    let missing = mux
        .send_photos_message(photos_message::Body::Send(SendPhoto { id: "2".into() }))
        .await
        .unwrap();
    assert!(!missing.available);

    let sent = mux
        .send_photos_message(photos_message::Body::Send(SendPhoto { id: "1".into() }))
        .await
        .unwrap();
    assert!(sent.available);
    eventually("the photo arrived", || low.files.lock().unwrap().len() == 1).await;
    let arrived = low.files.lock().unwrap()[0].path.clone();
    assert_eq!(std::fs::read(arrived).unwrap(), b"a photo");

    let just_taken = listed.photos[0].clone();
    let told = mux
        .send_photos_message(photos_message::Body::Taken(just_taken.clone()))
        .await
        .unwrap();
    assert!(told.available);
    assert_eq!(high.taken.lock().unwrap().as_slice(), [just_taken]);
}

#[tokio::test]
async fn a_computer_reads_the_phones_texts_and_replies() {
    let (low, high) = pair();
    low.registry
        .connect(HIGH, high.cert.spki_hash, high.listen_addr)
        .await
        .unwrap();
    let mux = low.registry.get(HIGH).unwrap();

    let listed = mux
        .send_messages_message(messages_message::Body::Conversations(ListConversations {
            limit: 1000,
        }))
        .await
        .unwrap();
    assert!(listed.available);
    assert_eq!(listed.conversations[0].name, "Asha");

    let read = mux
        .send_messages_message(messages_message::Body::Read(ReadConversation {
            conversation_id: "7".into(),
            limit: 1000,
        }))
        .await
        .unwrap();
    assert_eq!(read.messages[0].body, "See you at 6");

    let sent = mux
        .send_messages_message(messages_message::Body::Send(SendText {
            address: "+15550100".into(),
            body: "On my way".into(),
        }))
        .await
        .unwrap();
    assert!(sent.sent);
    assert_eq!(
        high.messages.sent.lock().unwrap().as_slice(),
        [("+15550100".to_string(), "On my way".to_string())]
    );

    let told = mux
        .send_messages_message(messages_message::Body::Changed(MessagesChanged {}))
        .await
        .unwrap();
    assert!(told.available);
    assert_eq!(*high.changes.lock().unwrap(), 1);
}

#[tokio::test]
async fn a_ringing_phone_shows_on_the_computer_which_answers_it() {
    let (phone, computer) = pair();
    phone
        .registry
        .connect(HIGH, computer.cert.spki_hash, computer.listen_addr)
        .await
        .unwrap();
    let to_computer = phone.registry.get(HIGH).unwrap();
    let ringing = Call {
        state: call::State::Ringing.into(),
        number: "+15550100".into(),
        name: "Asha".into(),
    };
    let shown = to_computer
        .send_calls_message(calls_message::Body::Call(ringing.clone()))
        .await
        .unwrap();
    assert!(shown.done);
    assert_eq!(computer.calls.lock().unwrap().as_slice(), [ringing]);

    eventually("the computer has the session", || {
        computer.registry.get(LOW).is_some()
    })
    .await;
    let to_phone = computer.registry.get(LOW).unwrap();
    let answer = |kind: call_action::Kind| {
        calls_message::Body::Action(CallAction {
            kind: kind.into(),
            ..Default::default()
        })
    };
    let answered = to_phone
        .send_calls_message(answer(call_action::Kind::Answer))
        .await
        .unwrap();
    assert!(answered.done);
    assert!(phone.phone.answered.load(Ordering::Relaxed));
    let again = to_phone
        .send_calls_message(answer(call_action::Kind::Answer))
        .await
        .unwrap();
    assert!(!again.done, "nothing left ringing");
}

#[tokio::test]
async fn a_computer_watches_the_phones_screen_and_taps_on_it() {
    let (computer, phone) = pair();
    computer
        .registry
        .connect(HIGH, phone.cert.spki_hash, phone.listen_addr)
        .await
        .unwrap();
    let to_phone = computer.registry.get(HIGH).unwrap();

    let request = ScreenRequest { max_size: 1280 };
    let (start, mut frames, mut control) = to_phone.watch_screen(request).await.unwrap().unwrap();
    assert_eq!((start.width, start.height), (576, 1280));
    let first = frames.next().await.unwrap();
    let second = frames.next().await.unwrap();
    assert!(first.key && !second.key, "frames arrive whole and in order");
    assert_eq!(frames.next().await.unwrap().audio, [1, 2, 3, 4]);

    let tap = ScreenInput {
        body: Some(screen_input::Body::Touch(Touch {
            action: touch::Action::Down.into(),
            x: 0.5,
            y: 0.25,
        })),
    };
    control.send(&tap).await.unwrap();
    eventually("the tap reaches the phone", || {
        phone.screen.controls.lock().unwrap().as_slice() == [tap.clone()]
    })
    .await;

    drop(control);
    eventually("closing stops the sharing", || {
        phone.screen.stopped.load(Ordering::Relaxed)
    })
    .await;
    assert!(frames.next().await.is_err());
}

#[tokio::test]
async fn a_computer_uses_the_phones_camera_and_switches_sides() {
    let (computer, phone) = pair();
    computer
        .registry
        .connect(HIGH, phone.cert.spki_hash, phone.listen_addr)
        .await
        .unwrap();
    let to_phone = computer.registry.get(HIGH).unwrap();

    let request = CameraRequest {
        max_size: 1920,
        front: true,
    };
    let (_, mut frames, mut control) = to_phone.watch_camera(request).await.unwrap().unwrap();
    assert!(frames.next().await.unwrap().key);

    let back = CameraControl {
        body: Some(camera_control::Body::Front(false)),
    };
    control.send(&back).await.unwrap();
    eventually("the switch reaches the phone", || {
        phone.camera.controls.lock().unwrap().as_slice() == [back]
    })
    .await;

    drop(control);
    eventually("closing stops the camera", || {
        phone.camera.stopped.load(Ordering::Relaxed)
    })
    .await;
}

#[tokio::test]
async fn the_computer_sees_what_the_phone_plays_and_skips_it() {
    let (phone, computer) = pair();
    phone
        .registry
        .connect(HIGH, computer.cert.spki_hash, computer.listen_addr)
        .await
        .unwrap();
    let song = NowPlaying {
        title: "Clair de lune".into(),
        playing: true,
        ..Default::default()
    };
    let to_computer = phone.registry.get(HIGH).unwrap();
    let shown = to_computer
        .send_media_message(media_message::Body::NowPlaying(song.clone()))
        .await
        .unwrap();
    assert!(shown.done);
    assert_eq!(computer.now_playing.lock().unwrap().as_slice(), [song]);

    eventually("the computer has the session", || {
        computer.registry.get(LOW).is_some()
    })
    .await;
    let skip = MediaCommand {
        kind: media_command::Kind::Next.into(),
    };
    let skipped = computer
        .registry
        .get(LOW)
        .unwrap()
        .send_media_message(media_message::Body::Command(skip))
        .await
        .unwrap();
    assert!(skipped.done);
    assert_eq!(
        phone.player.commands.lock().unwrap().as_slice(),
        [media_command::Kind::Next]
    );
}

#[tokio::test]
async fn the_computer_rings_the_phone_and_stops_it() {
    let (computer, phone) = pair();
    computer
        .registry
        .connect(HIGH, phone.cert.spki_hash, phone.listen_addr)
        .await
        .unwrap();
    let to_phone = computer.registry.get(HIGH).unwrap();

    assert!(to_phone.ring_peer(true).await.unwrap().done);
    assert!(phone.bell.0.load(Ordering::Relaxed));
    assert!(to_phone.ring_peer(false).await.unwrap().done);
    assert!(!phone.bell.0.load(Ordering::Relaxed));
}

#[tokio::test]
async fn the_computer_pointer_moves_onto_the_phone_and_comes_back() {
    let (computer, phone) = pair();
    computer
        .registry
        .connect(HIGH, phone.cert.spki_hash, phone.listen_addr)
        .await
        .unwrap();
    let to_phone = computer.registry.get(HIGH).unwrap();
    let start = PointerStart {
        y: 0.5,
        from_left: true,
    };
    let (mut control, mut leaves) = to_phone.point(start).await.unwrap().unwrap();

    let input = |body| PointerInput { body: Some(body) };
    let moved = |dx| input(pointer_input::Body::Move(PointerMove { dx, dy: 0.0 }));
    for sent in [
        moved(40.0),
        input(pointer_input::Body::Press(true)),
        input(pointer_input::Body::Press(false)),
        moved(-80.0),
    ] {
        control.send(&sent).await.unwrap();
    }
    assert_eq!(leaves.next().await.unwrap().y, 0.25);
    assert_eq!(phone.pad.inputs.lock().unwrap().len(), 4);
    assert_eq!(
        phone.pad.inputs.lock().unwrap()[1].body,
        Some(pointer_input::Body::Press(true))
    );

    drop(control);
    eventually("the phone hid its pointer", || {
        phone.pad.stopped.load(Ordering::Relaxed)
    })
    .await;
}

#[tokio::test]
async fn the_phone_locks_the_computer_and_types_into_it() {
    let (computer, phone) = pair();
    phone
        .registry
        .connect(LOW, computer.cert.spki_hash, computer.listen_addr)
        .await
        .unwrap();
    let to_computer = phone.registry.get(LOW).unwrap();

    let lock = computer_action::Body::Lock(true);
    let typed = computer_action::Body::TypeText("hello".to_string());
    assert!(to_computer.act_on_peer(lock.clone()).await.unwrap().done);
    assert!(to_computer.act_on_peer(typed.clone()).await.unwrap().done);
    assert_eq!(*computer.desk.0.lock().unwrap(), [lock, typed]);
}

#[tokio::test]
async fn snippets_pinned_on_one_device_reach_the_other() {
    let (computer, phone) = pair();
    phone
        .registry
        .connect(LOW, computer.cert.spki_hash, computer.listen_addr)
        .await
        .unwrap();
    let address = Snippet {
        id: "a".to_string(),
        text: "12 High Street".to_string(),
        changed_at: 1,
        removed: false,
    };

    let to_computer = phone.registry.get(LOW).unwrap();
    assert!(to_computer
        .send_snippets(vec![address.clone()])
        .await
        .unwrap());
    assert_eq!(*computer.pinned.0.lock().unwrap(), [address]);
}

#[tokio::test]
async fn the_computer_searches_the_phone_and_gets_only_matches() {
    let (computer, phone) = pair();
    computer
        .registry
        .connect(HIGH, phone.cert.spki_hash, phone.listen_addr)
        .await
        .unwrap();
    let to_phone = computer.registry.get(HIGH).unwrap();

    let found = to_phone.search_peer("dinner".to_string()).await.unwrap();
    assert!(found.available);
    assert_eq!(found.results.len(), 1);
    assert_eq!(found.results[0].title, "Sam");
    assert!(to_phone
        .search_peer("lunch".to_string())
        .await
        .unwrap()
        .results
        .is_empty());
}

#[tokio::test]
async fn the_computer_lists_the_phones_contacts() {
    let (computer, phone) = pair();
    computer
        .registry
        .connect(HIGH, phone.cert.spki_hash, phone.listen_addr)
        .await
        .unwrap();
    let to_phone = computer.registry.get(HIGH).unwrap();

    let reply = to_phone
        .send_messages_message(messages_message::Body::Contacts(ListContacts { limit: 10 }))
        .await
        .unwrap();
    assert!(reply.available);
    assert_eq!(reply.contacts[0].name, "Asha");
    assert!(reply.contacts[0].favorite);
}
