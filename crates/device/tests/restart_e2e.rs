// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Pair once, restart both devices, and the phone reconnects by itself and can send a
//! file to the computer.
//!
//! Each device runs on its own runtime, started the way the apps start one: keys, paired
//! devices and permissions come from its data folder, then it listens, advertises and dials.
//! Dropping the runtime stands in for quitting the app, so nothing carries over a restart
//! except what was saved to disk.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use device::{Device, Stores};
use identity::FileSecretStore;
use pairing::DeviceKeys;
use sessions::{SessionCapabilityHandlers, SessionState, ThisDevice};
use tokio::runtime::Runtime;

/// Long enough for a dial and a QUIC handshake on loopback, with room for a slow CI machine.
const WAIT: Duration = Duration::from_secs(20);

/// Every app here listens on 127.0.0.1, where only one can have the default port that paired
/// devices dial, so the tests take turns.
fn one_at_a_time() -> MutexGuard<'static, ()> {
    static NETWORK: Mutex<()> = Mutex::new(());
    NETWORK.lock().unwrap_or_else(|e| e.into_inner())
}

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
struct App {
    runtime: Runtime,
    device: Device,
    received: PathBuf,
}

impl App {
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
        let stores = Stores::open(dir.join("continue.db")).unwrap();
        let received = dir.join("received");
        let mut handlers = SessionCapabilityHandlers::new(&received)
            .with_this_device(ThisDevice::new(name, protocol::v1::Platform::Linux));
        let names = stores.clone();
        handlers.on_device_info = Some(Arc::new(move |peer, device| {
            names.rename_peer(peer, &device.name);
        }));

        let device = runtime.block_on(async {
            let device = Device::new(stores, keys, handlers, None).unwrap();
            let port = device.listen().unwrap().local_addr().unwrap().port();
            device.discover(port, 1);
            device
        });
        Self {
            runtime,
            device,
            received,
        }
    }

    fn fingerprint(&self) -> &str {
        &self.device.fingerprint
    }

    /// Quits the app: every task, socket and connection it had goes with its runtime.
    fn quit(self) {
        self.runtime.shutdown_background();
    }

    fn connected_to(&self, other: &App) -> bool {
        self.device.sessions.state(other.fingerprint()) == SessionState::Connected
    }

    /// The name this device shows for `other`.
    fn name_of(&self, other: &App) -> String {
        self.device.stores.peer_name(other.fingerprint())
    }
}

/// The computer shows a code and the phone scans it, as in the apps. The phone accepts last:
/// both listen on 127.0.0.1 here, so only the computer has the default port the phone dials
/// once it accepts, and the computer must already trust it by then.
fn pair(computer: &App, phone: &App) {
    let server = computer.runtime.block_on(async {
        computer
            .device
            .start_pairing(0, |port| format!("127.0.0.1:{port}"))
            .unwrap()
    });
    let on_phone = phone
        .runtime
        .block_on(phone.device.pair_nearby(&server.code))
        .unwrap();
    let phone_peer = computer
        .runtime
        .block_on(server.finish())
        .unwrap()
        .accept()
        .unwrap();
    let computer_peer = on_phone.accept().unwrap();

    assert_eq!(phone_peer.fingerprint, phone.fingerprint());
    assert_eq!(computer_peer.fingerprint, computer.fingerprint());
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
    let _turn = one_at_a_time();
    let (computer_dir, phone_dir) = (data_dir("computer"), data_dir("phone"));

    let computer = App::start(&computer_dir, "Work laptop");
    let phone = App::start(&phone_dir, "Sam's Pixel");
    let (computer_fingerprint, phone_fingerprint) = (
        computer.fingerprint().to_string(),
        phone.fingerprint().to_string(),
    );
    pair(&computer, &phone);
    eventually("the new pair connects", || {
        phone.connected_to(&computer) && computer.connected_to(&phone)
    });
    eventually("each knows the other's name", || {
        phone.name_of(&computer) == "Work laptop" && computer.name_of(&phone) == "Sam's Pixel"
    });

    phone.quit();
    computer.quit();

    // The computer was renamed while it was off.
    let computer = App::start(&computer_dir, "Studio desktop");
    let phone = App::start(&phone_dir, "Sam's Pixel");
    assert_eq!(
        computer.fingerprint(),
        computer_fingerprint,
        "the computer kept its identity"
    );
    assert_eq!(
        phone.fingerprint(),
        phone_fingerprint,
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
    let session = phone.device.sessions.get(computer.fingerprint()).unwrap();
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

#[test]
fn what_waits_for_a_device_survives_a_restart_and_stays_while_it_is_away() {
    let _turn = one_at_a_time();
    let dir = data_dir("waiting");
    let computer = App::start(&dir, "Work laptop");
    let first = computer
        .device
        .send_later("phone", history::Kind::Text, "hi")
        .unwrap();
    computer
        .device
        .send_later("phone", history::Kind::File, "/missing.jpg")
        .unwrap();
    computer.quit();

    let computer = App::start(&dir, "Work laptop");
    let done = std::sync::Mutex::new(Vec::new());
    computer
        .runtime
        .block_on(computer.device.send_waiting("phone", |item, sent| {
            done.lock().unwrap().push((item.id, sent))
        }));

    assert!(done.into_inner().unwrap().is_empty());
    let waiting = computer
        .device
        .stores
        .history
        .waiting(Some("phone"))
        .unwrap();
    assert_eq!(waiting.len(), 2);
    assert_eq!(waiting[0].id, first);
}

#[test]
fn a_nearby_pairing_shows_the_same_code_on_both_and_trusts_only_once_accepted() {
    let _turn = one_at_a_time();
    let (computer, phone) = (
        App::start(&data_dir("computer"), "Work laptop"),
        App::start(&data_dir("phone"), "Phone"),
    );
    let server = computer.runtime.block_on(async {
        computer
            .device
            .start_pairing(0, |port| format!("127.0.0.1:{port}"))
            .unwrap()
    });
    let on_phone = phone
        .runtime
        .block_on(phone.device.pair_nearby(&server.code))
        .unwrap();
    let on_computer = computer.runtime.block_on(server.finish()).unwrap();

    assert_eq!(on_phone.code(), on_computer.code());
    assert_eq!(on_phone.code().len(), 6);
    let trusted = |app: &App, other: &App| {
        app.device
            .stores
            .trust
            .get_peer(other.fingerprint())
            .unwrap()
            .is_some()
    };
    assert!(!trusted(&phone, &computer) && !trusted(&computer, &phone));

    on_phone.accept().unwrap();
    on_computer.accept().unwrap();
    assert!(trusted(&phone, &computer) && trusted(&computer, &phone));
}
