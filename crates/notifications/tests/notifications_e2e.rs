// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::net::SocketAddr;

use capabilities::CapabilityQuery;
use notifications::{NotificationAction, NotificationDispatcher, NotificationPost};
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

/// Both ends of a loopback QUIC connection whose peers pin each other's certificates.
async fn loopback_connections() -> (quinn::Connection, quinn::Connection) {
    let server_cert = TransportCertificate::generate().unwrap();
    let client_cert = TransportCertificate::generate().unwrap();
    let server_tls = server_cert
        .build_pinned_server_tls(client_cert.spki_hash)
        .unwrap();
    let client_tls = client_cert
        .build_pinned_client_tls(server_cert.spki_hash)
        .unwrap();

    let loopback: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server_endpoint = create_server_endpoint(loopback, server_tls).unwrap();
    let client_endpoint = create_client_endpoint(loopback, client_tls).unwrap();

    let connecting = client_endpoint
        .connect(server_endpoint.local_addr().unwrap(), "continue-device")
        .unwrap();
    let accepting = async { server_endpoint.accept().await.expect("incoming conn").await };
    let (client, server) = tokio::join!(connecting, accepting);
    (client.unwrap(), server.unwrap())
}

#[tokio::test]
async fn notifications_e2e_post_and_action() {
    let (client_conn, server_conn) = loopback_connections().await;

    let dispatcher = NotificationDispatcher::new();

    // Receiver task. It gets a clone so `server_conn` keeps the connection open
    // until the sender has read the reply.
    let receiver_conn = server_conn.clone();
    let recv_handle = tokio::spawn(async move {
        let (mut send, mut recv) = receiver_conn.accept_bi().await.expect("bi stream");

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

    let (client_conn, _server_conn) = loopback_connections().await;
    let (mut send, mut recv) = client_conn.open_bi().await.unwrap();

    // Rejected before anything is written to the stream.
    let res = dispatcher.send_post(
        &mut send,
        &mut recv,
        oversized_post,
        &query,
    ).await;

    assert!(matches!(res, Err(notifications::NotificationError::BodyTooLarge { .. })));
}
