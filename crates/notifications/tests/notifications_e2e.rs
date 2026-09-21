// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::net::SocketAddr;

use capabilities::CapabilityQuery;
use notifications::{
    NotificationAction, NotificationActionInvoke, NotificationDismiss, NotificationDispatcher,
    NotificationPost,
};
use protocol::CapabilityId;
use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

fn authorized_notification_query() -> CapabilityQuery {
    let mut caps = HashSet::new();
    caps.insert(CapabilityId::NOTIFICATIONS);
    CapabilityQuery {
        capability: CapabilityId::NOTIFICATIONS,
        is_os_available: true,
        is_app_permitted: true,
        is_peer_authorized: true,
        negotiated_session_capabilities: caps,
    }
}

#[tokio::test]
async fn notifications_e2e_post_and_action() {
    let server_cert = TransportCertificate::generate().unwrap();
    let client_cert = TransportCertificate::generate().unwrap();

    let server_tls = server_cert
        .build_pinned_server_tls(client_cert.spki_hash)
        .unwrap();
    let client_tls = client_cert
        .build_pinned_client_tls(server_cert.spki_hash)
        .unwrap();

    let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server_endpoint = create_server_endpoint(server_addr, server_tls).unwrap();
    let bound_addr = server_endpoint.local_addr().unwrap();

    let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let client_endpoint = create_client_endpoint(client_addr, client_tls).unwrap();

    let dispatcher = NotificationDispatcher::new();

    // Receiver task
    let recv_handle = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.expect("incoming conn");
        let conn = incoming.await.expect("conn");
        let (mut send, mut recv) = conn.accept_bi().await.expect("bi stream");

        let query = authorized_notification_query();
        let post = dispatcher
            .receive_post(&mut send, &mut recv, &query, |p| {
                assert_eq!(p.title, "Alice");
                assert_eq!(p.body, "Hello from phone!");
                assert_eq!(p.actions.len(), 1);
                Ok(())
            })
            .await
            .expect("receive post");

        assert_eq!(post.notification_id, "notif-123");
    });

    // Sender
    let client_conn = client_endpoint
        .connect(bound_addr, "continue-device")
        .unwrap()
        .await
        .unwrap();
    let (mut client_send, mut client_recv) = client_conn.open_bi().await.unwrap();

    let sender_dispatcher = NotificationDispatcher::new();
    let query = authorized_notification_query();
    let post = NotificationPost {
        notification_id: "notif-123".to_string(),
        package_name: "org.continue.chat".to_string(),
        app_name: "Chat".to_string(),
        title: "Alice".to_string(),
        body: "Hello from phone!".to_string(),
        timestamp: 123456789,
        actions: vec![NotificationAction {
            action_id: "reply".to_string(),
            label: "Reply".to_string(),
            is_reply: true,
        }],
    };

    let ack = sender_dispatcher
        .send_post(&mut client_send, &mut client_recv, post, &query)
        .await
        .expect("send post");

    assert!(ack.handled);
    recv_handle.await.unwrap();
}

#[tokio::test]
async fn notification_rejects_oversized_body() {
    let dispatcher = NotificationDispatcher::new();
    let query = authorized_notification_query();

    let oversized_post = NotificationPost {
        notification_id: "notif-huge".to_string(),
        package_name: "org.continue.chat".to_string(),
        app_name: "Chat".to_string(),
        title: "Alice".to_string(),
        body: "A".repeat(5000), // Exceeds MAX_NOTIFICATION_BODY_BYTES (4096)
        timestamp: 123456789,
        actions: vec![],
    };

    let res = dispatcher.send_post(
        &mut unsafe { std::mem::zeroed() },
        &mut unsafe { std::mem::zeroed() },
        oversized_post,
        &query,
    ).await;

    assert!(matches!(res, Err(notifications::NotificationError::BodyTooLarge { .. })));
}
