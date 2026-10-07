// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

mod common;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use capabilities::CapabilityQuery;
use clipboard::ClipboardFormat;
use common::{link, scratch_dir, send_sized, Link, PHONE};
use permissions::PermissionStore;
use protocol::CapabilityId;
use sessions::SessionCapabilityHandlers;

fn handlers_with(store: &PermissionStore) -> (SessionCapabilityHandlers, PathBuf) {
    let downloads = scratch_dir();
    let handlers =
        SessionCapabilityHandlers::new(&downloads).with_permission_store(Arc::new(store.clone()));
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
async fn a_paired_device_may_send_without_being_asked() {
    let store = PermissionStore::in_memory().unwrap();
    let (mut handlers, downloads) = handlers_with(&store);
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = received.clone();
    handlers.on_clipboard_received = Some(Arc::new(move |_peer, update| {
        sink.lock().unwrap().push(update.payload);
    }));
    let link = link(handlers).await;

    assert!(send_file(&link, "boarding pass.pdf").await);
    assert!(downloads.join("boarding pass.pdf").exists());
    assert!(send_text(&link, "Gate B12").await);
    assert_eq!(*received.lock().unwrap(), vec![b"Gate B12".to_vec()]);
}

#[tokio::test]
async fn a_feature_turned_off_for_a_device_is_refused_and_writes_nothing() {
    let store = PermissionStore::in_memory().unwrap();
    store
        .set_allowed(PHONE, CapabilityId::FILE_TRANSFER, false)
        .unwrap();
    let (handlers, downloads) = handlers_with(&store);
    let link = link(handlers).await;

    assert!(!send_file(&link, "unwanted.zip").await);
    assert_eq!(std::fs::read_dir(downloads).unwrap().count(), 0);
    assert!(send_text(&link, "still fine").await);
}

#[tokio::test]
async fn showing_notifications_can_be_turned_off_but_replies_and_mutes_still_work() {
    let store = PermissionStore::in_memory().unwrap();
    store
        .set_allowed(PHONE, CapabilityId::NOTIFICATIONS, false)
        .unwrap();
    let (mut handlers, _) = handlers_with(&store);
    let heard = Arc::new(Mutex::new(Vec::new()));
    let record = heard.clone();
    handlers.on_notification = Some(Arc::new(move |_peer, body| {
        record.lock().unwrap().push(body);
    }));
    let link = link(handlers).await;
    let query = CapabilityQuery::negotiated(CapabilityId::NOTIFICATIONS, true);

    let post = notifications::Body::Post(notifications::NotificationPost {
        notification_id: "chat-1".to_string(),
        title: "Alice".to_string(),
        ..Default::default()
    });
    let reply = notifications::Body::Action(notifications::NotificationActionInvoke {
        notification_id: "chat-1".to_string(),
        action_id: "reply".to_string(),
        reply_text: "On my way".to_string(),
    });

    assert!(link
        .phone
        .send_notification_to_peer(post, &query)
        .await
        .is_err());
    let mute = notifications::Body::Mute(notifications::NotificationMute {
        package_name: "com.chat".to_string(),
    });
    for body in [reply.clone(), mute.clone()] {
        assert!(link
            .phone
            .send_notification_to_peer(body, &query)
            .await
            .is_ok());
    }
    assert_eq!(heard.lock().unwrap().as_slice(), [reply, mute]);
}
