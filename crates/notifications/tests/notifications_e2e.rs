// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

use capabilities::CapabilityQuery;
use notifications::{Body, NotificationAction, NotificationActionInvoke, NotificationPost};
use protocol::CapabilityId;
use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

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

fn post(body: &str) -> NotificationPost {
    NotificationPost {
        notification_id: "notif-123".to_string(),
        package_name: "org.continue.chat".to_string(),
        app_name: "Chat".to_string(),
        title: "Alice".to_string(),
        body: body.to_string(),
        timestamp: 123456789,
        actions: vec![NotificationAction {
            action_id: "reply".to_string(),
            label: "Reply".to_string(),
            is_reply: true,
        }],
    }
}

#[tokio::test]
async fn a_post_and_a_reply_to_it_each_arrive_as_what_they_are() {
    let (client_conn, server_conn) = loopback_connections().await;
    let query = CapabilityQuery::negotiated(CapabilityId::NOTIFICATIONS, true);

    // The receiver gets a clone so `server_conn` keeps the connection open
    // until the sender has read the reply.
    let receiver_conn = server_conn.clone();
    let received = tokio::spawn(async move {
        let mut bodies = Vec::new();
        for _ in 0..2 {
            let (mut send, mut recv) = receiver_conn.accept_bi().await.expect("bi stream");
            bodies.push(notifications::read(&mut recv).await.expect("read"));
            notifications::acknowledge(&mut send, true)
                .await
                .expect("ack");
        }
        bodies
    });

    let reply = NotificationActionInvoke {
        notification_id: "notif-123".to_string(),
        action_id: "reply".to_string(),
        reply_text: "On my way".to_string(),
    };
    for body in [
        Body::Post(post("Hello from phone!")),
        Body::Action(reply.clone()),
    ] {
        let (mut send, mut recv) = client_conn.open_bi().await.unwrap();
        notifications::send(&mut send, &mut recv, body, &query)
            .await
            .expect("taken");
    }

    let bodies = received.await.unwrap();
    assert_eq!(bodies[0], Body::Post(post("Hello from phone!")));
    assert_eq!(bodies[1], Body::Action(reply));
}

#[tokio::test]
async fn a_refused_notification_is_an_error_for_the_sender() {
    let (client_conn, server_conn) = loopback_connections().await;
    let receiver_conn = server_conn.clone();
    tokio::spawn(async move {
        let (mut send, mut recv) = receiver_conn.accept_bi().await.expect("bi stream");
        notifications::read(&mut recv).await.expect("read");
        notifications::acknowledge(&mut send, false)
            .await
            .expect("ack");
    });

    let (mut send, mut recv) = client_conn.open_bi().await.unwrap();
    let query = CapabilityQuery::negotiated(CapabilityId::NOTIFICATIONS, true);
    let sent = notifications::send(&mut send, &mut recv, Body::Post(post("Hi")), &query).await;
    assert!(sent.is_err());
}

#[tokio::test]
async fn notification_rejects_oversized_body() {
    let (client_conn, _server_conn) = loopback_connections().await;
    let (mut send, mut recv) = client_conn.open_bi().await.unwrap();
    let query = CapabilityQuery::negotiated(CapabilityId::NOTIFICATIONS, true);

    // Rejected before anything is written to the stream.
    let sent = notifications::send(
        &mut send,
        &mut recv,
        Body::Post(post(&"A".repeat(5000))),
        &query,
    )
    .await;

    assert!(matches!(
        sent,
        Err(notifications::NotificationError::BodyTooLarge { .. })
    ));
}
