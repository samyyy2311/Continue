// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! A phone linked to a computer over loopback, shared by the end-to-end tests.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use sessions::{spawn_capabilities_dispatcher, SessionCapabilityHandlers, SessionMultiplexer};
use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

pub const PHONE: &str = "phone-fingerprint";

pub fn scratch_dir() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "continue-sessions-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub struct Link {
    /// The phone's side, used to send to the computer.
    pub phone: Arc<SessionMultiplexer>,
    _computer: Arc<SessionMultiplexer>,
}

/// Connects a phone to a computer whose incoming streams go through `handlers`.
pub async fn link(handlers: SessionCapabilityHandlers) -> Link {
    let computer_cert = TransportCertificate::generate().unwrap();
    let phone_cert = TransportCertificate::generate().unwrap();
    let server_tls = computer_cert
        .build_pinned_server_tls(phone_cert.spki_hash)
        .unwrap();
    let client_tls = phone_cert
        .build_pinned_client_tls(computer_cert.spki_hash)
        .unwrap();

    let any: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = create_server_endpoint(any, server_tls).unwrap();
    let server_addr = server.local_addr().unwrap();
    let client = create_client_endpoint(any, client_tls).unwrap();

    let accept = tokio::spawn(async move {
        let conn = server.accept().await.unwrap().await.unwrap();
        let computer = Arc::new(SessionMultiplexer::new(PHONE.to_string(), conn));
        spawn_capabilities_dispatcher(computer.clone(), handlers, 8);
        computer
    });
    let conn = client
        .connect(server_addr, "continue-device")
        .unwrap()
        .await
        .unwrap();
    Link {
        phone: Arc::new(SessionMultiplexer::new("computer".to_string(), conn)),
        _computer: accept.await.unwrap(),
    }
}

/// Sends a file of `size` bytes from the phone; true if the computer took it.
pub async fn send_sized(link: &Link, name: &str, size: usize) -> bool {
    let source = scratch_dir().join(name);
    std::fs::write(&source, vec![b'x'; size]).unwrap();
    link.phone
        .send_file_to_peer(&source, format!("tx-{name}"), None::<fn(u64, u64)>)
        .await
        .is_ok()
}
