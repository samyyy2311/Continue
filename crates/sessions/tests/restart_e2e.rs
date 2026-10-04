// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Phase 1's finish line: pair once, restart both devices, and the phone reconnects by
//! itself and can send a file to the computer.
//!
//! Each device runs on its own runtime, started the way the apps start one: keys, paired
//! devices and permissions come from its data folder, then it listens, advertises and dials.
//! Dropping the runtime stands in for quitting the app, so nothing carries over a restart
//! except what was saved to disk.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use identity::{FileSecretStore, Fingerprint};
use pairing::{DeviceKeys, InitiatorPairing, ReplayCache, ResponderPairing, TrustStore};
use permissions::{PermissionStore, PersistedGrant};
use protocol::CapabilityId;
use sessions::{
    accept_peers, connect_paired_peers, listen_for_peers, remember_peer_address, RegistryConfig,
    SessionCapabilityHandlers, SessionRegistry, SessionState, ThisDevice,
};
use tokio::runtime::Runtime;
use transport::DialConfig;

/// Long enough for a dial and a QUIC handshake on loopback, with room for a slow CI machine.
const WAIT: Duration = Duration::from_secs(20);

fn data_dir(name: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "continue-restart-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// One running copy of the app.
struct Device {
    runtime: Runtime,
    fingerprint: String,
    keys: DeviceKeys,
    trust_store: TrustStore,
    permissions: Arc<PermissionStore>,
    trusted_keys: Arc<RwLock<HashSet<[u8; 32]>>>,
    registry: SessionRegistry,
    received: PathBuf,
}

impl Device {
    /// Starts the app from what `dir` holds, creating it on first run, as a device called
    /// `name`.
    fn start(dir: &Path, name: &str) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let keys = DeviceKeys::load_or_create(&FileSecretStore::new(dir.join("secrets")).unwrap())
            .unwrap();
        let db = dir.join("continue.db");
        let trust_store = TrustStore::open(&db).unwrap();
        let permissions = Arc::new(PermissionStore::open(&db).unwrap());
        let fingerprint =
            Fingerprint::from_verifying_key(&keys.identity_signer.verifying_key().unwrap())
                .to_string();
        let trusted_keys = Arc::new(RwLock::new(
            trust_store
                .list_peers()
                .unwrap()
                .iter()
                .map(|peer| peer.transport_spki_hash)
                .collect(),
        ));
        let received = dir.join("received");
        let mut handlers = SessionCapabilityHandlers::new(&received)
            .with_permission_store(permissions.clone())
            .with_this_device(ThisDevice::new(name, protocol::v1::Platform::Linux));
        let names = trust_store.clone();
        handlers.on_device_info = Some(Arc::new(move |peer, device| {
            names.set_display_name(peer, &device.name).unwrap();
        }));

        // The registry spawns onto the runtime it's created in.
        let registry = runtime.block_on(async {
            SessionRegistry::new(
                fingerprint.clone(),
                keys.transport_cert.clone(),
                handlers,
                RegistryConfig::default(),
                None,
            )
        });
        runtime.block_on(async {
            let endpoint = listen_for_peers(&keys.transport_cert, trusted_keys.clone()).unwrap();
            let port = endpoint.local_addr().unwrap().port();
            tokio::spawn(discovery::advertise(port, 1));
            tokio::spawn(connect_paired_peers(registry.clone(), trust_store.clone()));
            tokio::spawn(accept_peers(
                endpoint,
                registry.clone(),
                trust_store.clone(),
            ));
        });

        Self {
            runtime,
            fingerprint,
            keys,
            trust_store,
            permissions,
            trusted_keys,
            registry,
            received,
        }
    }

    /// Quits the app: every task, socket and connection it had goes with its runtime.
    fn quit(self) {
        self.runtime.shutdown_background();
    }

    /// What the apps do once pairing succeeds.
    fn paired_with(&self, peer: &pairing::TrustedPeer, address: std::net::IpAddr) {
        self.trusted_keys
            .write()
            .unwrap()
            .insert(peer.transport_spki_hash);
        remember_peer_address(&self.trust_store, &peer.fingerprint, address);
        self.registry.redial_now();
    }

    fn connected_to(&self, other: &Device) -> bool {
        self.registry.state(&other.fingerprint) == SessionState::Connected
    }

    /// The name this device shows for `other`.
    fn name_of(&self, other: &Device) -> String {
        self.trust_store
            .get_peer(&other.fingerprint)
            .unwrap()
            .map(|peer| peer.display_name)
            .unwrap_or_default()
    }
}

/// The computer shows a code and the phone scans it, as in the apps.
fn pair(computer: &Device, phone: &Device) {
    let recorded = Arc::new(Mutex::new(None));
    let server_tls = computer
        .keys
        .transport_cert
        .build_pairing_server_tls(recorded.clone())
        .unwrap();
    let mut initiator = InitiatorPairing::new(
        computer.keys.identity_signer.clone(),
        computer.keys.transport_cert.clone(),
        computer.trust_store.clone(),
        Arc::new(ReplayCache::new()),
    );

    let (server, qr) = computer.runtime.block_on(async {
        let server =
            transport::create_server_endpoint("127.0.0.1:0".parse().unwrap(), server_tls).unwrap();
        let qr = initiator
            .generate_qr(server.local_addr().unwrap().to_string())
            .unwrap();
        (server, qr)
    });
    let computer_side = computer.runtime.spawn(async move {
        let connection = server.accept().await.unwrap().await.unwrap();
        let (mut send, mut recv) = connection.accept_bi().await.unwrap();
        let phone_spki = recorded.lock().unwrap().unwrap();
        let peer = initiator
            .complete_handshake(&mut send, &mut recv, phone_spki)
            .await
            .unwrap();
        (peer, connection.remote_address().ip())
    });

    let (computer_peer, computer_ip) = phone.runtime.block_on(async {
        let address = qr.endpoint.parse::<std::net::SocketAddr>().unwrap();
        let connection = transport::connect_pinned(
            &phone.keys.transport_cert,
            qr.transport_spki_hash,
            address,
            &DialConfig::default(),
        )
        .await
        .unwrap();
        let (mut send, mut recv) = connection.open_bi().await.unwrap();
        let peer = ResponderPairing::new(
            phone.keys.identity_signer.clone(),
            phone.keys.transport_cert.clone(),
            phone.trust_store.clone(),
        )
        .complete_handshake(&qr, &mut send, &mut recv)
        .await
        .unwrap();
        (peer, address.ip())
    });
    let (phone_peer, phone_ip) = computer.runtime.block_on(computer_side).unwrap();

    assert_eq!(phone_peer.fingerprint, phone.fingerprint);
    assert_eq!(computer_peer.fingerprint, computer.fingerprint);
    computer.paired_with(&phone_peer, phone_ip);
    phone.paired_with(&computer_peer, computer_ip);
}

fn eventually(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn paired_devices_reconnect_after_both_restart_and_the_phone_can_send_a_file() {
    let (computer_dir, phone_dir) = (data_dir("computer"), data_dir("phone"));

    let computer = Device::start(&computer_dir, "Work laptop");
    let phone = Device::start(&phone_dir, "Sam's Pixel");
    let (computer_fingerprint, phone_fingerprint) =
        (computer.fingerprint.clone(), phone.fingerprint.clone());
    pair(&computer, &phone);
    // The person allows files from the phone, as they would in the computer's settings.
    computer
        .permissions
        .set_persisted_grant(
            &phone.fingerprint,
            CapabilityId::FILE_TRANSFER,
            1,
            PersistedGrant::Allow,
        )
        .unwrap();
    eventually("the new pair connects", || {
        phone.connected_to(&computer) && computer.connected_to(&phone)
    });
    eventually("each knows the other's name", || {
        phone.name_of(&computer) == "Work laptop" && computer.name_of(&phone) == "Sam's Pixel"
    });

    phone.quit();
    computer.quit();

    // The computer was renamed while it was off.
    let computer = Device::start(&computer_dir, "Studio desktop");
    let phone = Device::start(&phone_dir, "Sam's Pixel");
    assert_eq!(
        computer.fingerprint, computer_fingerprint,
        "the computer kept its identity"
    );
    assert_eq!(
        phone.fingerprint, phone_fingerprint,
        "the phone kept its identity"
    );
    eventually("both restarted devices reconnect on their own", || {
        phone.connected_to(&computer) && computer.connected_to(&phone)
    });
    eventually("the phone shows the computer's new name", || {
        phone.name_of(&computer) == "Studio desktop"
    });
    assert_eq!(
        computer.name_of(&phone),
        "Sam's Pixel",
        "kept across the restart"
    );

    let photo = data_dir("photo").join("photo.jpg");
    std::fs::write(&photo, b"not really a photo").unwrap();
    let session = phone.registry.get(&computer.fingerprint).unwrap();
    let sent = phone.runtime.block_on(session.send_file_to_peer(
        &photo,
        "tx-photo".to_string(),
        None::<fn(u64, u64)>,
    ));
    assert_eq!(sent.unwrap(), 18);
    eventually("the computer saved the file", || {
        std::fs::read(computer.received.join("photo.jpg")).is_ok_and(|b| b == b"not really a photo")
    });

    phone.quit();
    computer.quit();
}
