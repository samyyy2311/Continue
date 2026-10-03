// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use capabilities::CapabilityQuery;
use clipboard::ClipboardFormat;
use permissions::{PermissionState, PermissionStore, PersistedGrant};
use protocol::CapabilityId;
use sessions::{
    spawn_capabilities_dispatcher, PermissionDecision, PermissionPrompt, PermissionRequest,
    SessionCapabilityHandlers, SessionMultiplexer,
};
use tokio::sync::oneshot;
use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

const PHONE: &str = "phone-fingerprint";

fn scratch_dir() -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "continue-permissions-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A prompt that records each question and answers it straight away.
fn answering(
    decision: PermissionDecision,
) -> (PermissionPrompt, Arc<Mutex<Vec<PermissionRequest>>>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let record = asked.clone();
    let prompt: PermissionPrompt = Arc::new(move |request| {
        record.lock().unwrap().push(request);
        let (answer, receiver) = oneshot::channel();
        let _ = answer.send(decision);
        receiver
    });
    (prompt, asked)
}

struct Link {
    /// The phone's side, used to send to the computer.
    phone: Arc<SessionMultiplexer>,
    _computer: Arc<SessionMultiplexer>,
}

/// Connects a phone to a computer whose incoming streams go through `handlers`.
async fn link(handlers: SessionCapabilityHandlers) -> Link {
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

fn handlers_with(
    store: &PermissionStore,
    prompt: Option<PermissionPrompt>,
) -> (SessionCapabilityHandlers, PathBuf) {
    let downloads = scratch_dir();
    let mut handlers =
        SessionCapabilityHandlers::new(&downloads).with_permission_store(Arc::new(store.clone()));
    if let Some(prompt) = prompt {
        handlers = handlers.with_permission_prompt(prompt);
    }
    (handlers, downloads)
}

async fn send_file(link: &Link, name: &str) -> bool {
    let source = scratch_dir().join(name);
    std::fs::write(&source, b"holiday photo").unwrap();
    link.phone
        .send_file_to_peer(&source, format!("tx-{name}"), None::<fn(u64, u64)>)
        .await
        .is_ok()
}

async fn send_text(link: &Link, text: &str) -> bool {
    let query = CapabilityQuery {
        capability: CapabilityId::CLIPBOARD,
        is_os_available: true,
        is_app_permitted: true,
        is_peer_authorized: true,
        negotiated_session_capabilities: HashSet::from([CapabilityId::CLIPBOARD]),
    };
    link.phone
        .send_clipboard_to_peer(ClipboardFormat::TextPlain, text.as_bytes().to_vec(), &query)
        .await
        .is_ok()
}

#[tokio::test]
async fn ask_shows_the_file_name_and_allowing_once_asks_again_next_time() {
    let store = PermissionStore::in_memory().unwrap();
    let (prompt, asked) = answering(PermissionDecision::Allow);
    let (handlers, downloads) = handlers_with(&store, Some(prompt));
    let link = link(handlers).await;

    assert!(send_file(&link, "first.jpg").await);
    assert!(send_file(&link, "second.jpg").await);

    let asked = asked.lock().unwrap();
    assert_eq!(
        *asked,
        vec![
            PermissionRequest {
                peer: PHONE.to_string(),
                capability: CapabilityId::FILE_TRANSFER,
                detail: Some("first.jpg".to_string()),
            },
            PermissionRequest {
                peer: PHONE.to_string(),
                capability: CapabilityId::FILE_TRANSFER,
                detail: Some("second.jpg".to_string()),
            },
        ]
    );
    assert!(downloads.join("first.jpg").exists());
}

#[tokio::test]
async fn always_allow_is_saved_and_not_asked_again() {
    let store = PermissionStore::in_memory().unwrap();
    let (prompt, asked) = answering(PermissionDecision::AlwaysAllow);
    let (handlers, _) = handlers_with(&store, Some(prompt));
    let link = link(handlers).await;

    assert!(send_file(&link, "one.pdf").await);
    assert!(send_file(&link, "two.pdf").await);

    assert_eq!(asked.lock().unwrap().len(), 1);
    assert_eq!(
        store
            .query_state(PHONE, CapabilityId::FILE_TRANSFER)
            .unwrap(),
        PermissionState::Allow
    );
}

#[tokio::test]
async fn declining_refuses_the_file_and_writes_nothing() {
    let store = PermissionStore::in_memory().unwrap();
    let (prompt, _) = answering(PermissionDecision::Decline);
    let (handlers, downloads) = handlers_with(&store, Some(prompt));
    let link = link(handlers).await;

    assert!(!send_file(&link, "unwanted.zip").await);
    assert_eq!(std::fs::read_dir(downloads).unwrap().count(), 0);
}

#[tokio::test]
async fn blocked_devices_are_refused_without_asking() {
    let store = PermissionStore::in_memory().unwrap();
    store
        .set_persisted_grant(PHONE, CapabilityId::CLIPBOARD, 1, PersistedGrant::Deny)
        .unwrap();
    let (prompt, asked) = answering(PermissionDecision::Allow);
    let (handlers, _) = handlers_with(&store, Some(prompt));
    let link = link(handlers).await;

    assert!(!send_text(&link, "hello").await);
    assert!(asked.lock().unwrap().is_empty());
}

#[tokio::test]
async fn ask_without_a_way_to_ask_refuses() {
    let store = PermissionStore::in_memory().unwrap();
    let (handlers, _) = handlers_with(&store, None);
    let link = link(handlers).await;

    assert!(!send_text(&link, "hello").await);
}

#[tokio::test]
async fn text_is_asked_about_and_delivered_once_allowed() {
    let store = PermissionStore::in_memory().unwrap();
    let (prompt, asked) = answering(PermissionDecision::Allow);
    let (mut handlers, _) = handlers_with(&store, Some(prompt));
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = received.clone();
    handlers.on_clipboard_received = Some(Arc::new(move |update| {
        sink.lock().unwrap().push(update.payload);
    }));
    let link = link(handlers).await;

    assert!(send_text(&link, "Gate B12").await);

    let asked = asked.lock().unwrap();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].capability, CapabilityId::CLIPBOARD);
    assert_eq!(asked[0].detail, None);
    assert_eq!(*received.lock().unwrap(), vec![b"Gate B12".to_vec()]);
}
