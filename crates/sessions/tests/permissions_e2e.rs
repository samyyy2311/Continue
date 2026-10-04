// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

mod common;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use capabilities::CapabilityQuery;
use clipboard::ClipboardFormat;
use common::{link, scratch_dir, send_sized, Link, PHONE};
use permissions::{PermissionState, PermissionStore, PersistedGrant};
use protocol::CapabilityId;
use sessions::{
    PermissionDecision, PermissionPrompt, PermissionRequest, SessionCapabilityHandlers,
};
use tokio::sync::oneshot;

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

/// Sends a small file from the phone; true if the computer took it.
async fn send_file(link: &Link, name: &str) -> bool {
    send_sized(link, name, 13).await
}

async fn send_text(link: &Link, text: &str) -> bool {
    let query = CapabilityQuery::negotiated(CapabilityId::CLIPBOARD, true);
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

    let asked: Vec<_> = asked
        .lock()
        .unwrap()
        .iter()
        .map(|q| (q.peer.clone(), q.capability, q.detail.clone()))
        .collect();
    assert_eq!(
        asked,
        [
            (
                PHONE.to_string(),
                CapabilityId::FILE_TRANSFER,
                Some("first.jpg".to_string())
            ),
            (
                PHONE.to_string(),
                CapabilityId::FILE_TRANSFER,
                Some("second.jpg".to_string())
            ),
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
    handlers.on_clipboard_received = Some(Arc::new(move |_peer, update| {
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
