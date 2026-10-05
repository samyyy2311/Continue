// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use protocol::v1::{session_envelope::Body, DisconnectReason, Ping, Pong, SessionEnvelope};
use protocol::CapabilityId;
use sessions::{open_capability_stream, Session, SessionMultiplexer};
use std::net::SocketAddr;
use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

#[tokio::test]
async fn session_multiplexer_routes_capability_streams() {
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

    let server_handle = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.expect("incoming conn");
        let conn = incoming.await.expect("conn");
        let mux = SessionMultiplexer::new("client-fingerprint".to_string(), conn);
        let mut rx = mux.spawn_router(8);

        let stream1 = rx.recv().await.expect("receive stream 1");
        assert_eq!(stream1.capability, CapabilityId::FILE_TRANSFER);

        let stream2 = rx.recv().await.expect("receive stream 2");
        assert_eq!(stream2.capability, CapabilityId::CLIPBOARD);

        mux
    });

    let client_conn = client_endpoint
        .connect(bound_addr, "continue-device")
        .unwrap()
        .await
        .unwrap();

    let client_mux = SessionMultiplexer::new("server-fingerprint".to_string(), client_conn);

    let (mut send1, _recv1) = client_mux
        .open_stream(CapabilityId::FILE_TRANSFER)
        .await
        .unwrap();
    send1.finish().unwrap();

    let (mut send2, _recv2) = client_mux
        .open_stream(CapabilityId::CLIPBOARD)
        .await
        .unwrap();
    send2.finish().unwrap();

    let _server_mux = server_handle.await.unwrap();
}

#[tokio::test]
async fn session_control_ping_pong_roundtrip() {
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

    // Server accepts control stream and responds to Ping with Pong
    let server_handle = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.expect("incoming conn");
        let conn = incoming.await.expect("conn");
        let mux = SessionMultiplexer::new("client-fingerprint".to_string(), conn);
        let _ = mux.spawn_router(8);

        // Keep connection open until client test completes
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        mux
    });

    let client_conn = client_endpoint
        .connect(bound_addr, "continue-device")
        .unwrap()
        .await
        .unwrap();

    // Open control stream manually and verify Ping -> Pong
    let (mut send, mut recv) = open_capability_stream(&client_conn, CapabilityId::CONTROL)
        .await
        .unwrap();

    let ping = SessionEnvelope {
        body: Some(Body::Ping(Ping { seq: 42 })),
    };
    Session::send_envelope(&mut send, &ping).await.unwrap();

    let resp = Session::read_envelope(&mut recv).await.unwrap();
    match resp.body {
        Some(Body::Pong(Pong { seq })) => assert_eq!(seq, 42),
        other => panic!("Expected Pong, got {other:?}"),
    }

    let server_mux = server_handle.await.unwrap();
    server_mux
        .disconnect(DisconnectReason::Normal, "test complete".to_string())
        .await;
}

#[tokio::test]
async fn notifications_routing_and_quick_reply_roundtrip() {
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

    let (notif_tx, mut notif_rx) = tokio::sync::mpsc::channel(1);
    let (action_tx, mut action_rx) = tokio::sync::mpsc::channel(1);
    let (dismiss_tx, mut dismiss_rx) = tokio::sync::mpsc::channel(1);

    // Desktop side (server): receives notifications & dismissals, sends actions
    let server_handle = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.expect("incoming conn");
        let conn = incoming.await.expect("conn");
        let mux = std::sync::Arc::new(SessionMultiplexer::new(
            "phone-fingerprint".to_string(),
            conn,
        ));

        let mut handlers = sessions::SessionCapabilityHandlers::new(std::env::temp_dir());
        let n_tx = notif_tx.clone();
        handlers.on_notification_received = Some(std::sync::Arc::new(move |_peer, post| {
            let _ = n_tx.try_send(post);
        }));
        let d_tx = dismiss_tx.clone();
        handlers.on_notification_dismiss = Some(std::sync::Arc::new(move |_peer, dismiss| {
            let _ = d_tx.try_send(dismiss);
        }));

        sessions::spawn_capabilities_dispatcher(mux.clone(), handlers, 8);
        mux
    });

    let client_conn = client_endpoint
        .connect(bound_addr, "continue-device")
        .unwrap()
        .await
        .unwrap();

    // Phone side (client): sends notifications, receives actions
    let client_mux = std::sync::Arc::new(SessionMultiplexer::new(
        "desktop-fingerprint".to_string(),
        client_conn,
    ));
    let mut client_handlers = sessions::SessionCapabilityHandlers::new(std::env::temp_dir());
    let a_tx = action_tx.clone();
    client_handlers.on_notification_action = Some(std::sync::Arc::new(move |_peer, action| {
        let _ = a_tx.try_send(action);
    }));
    sessions::spawn_capabilities_dispatcher(client_mux.clone(), client_handlers, 8);

    let query = capabilities::CapabilityQuery::negotiated(CapabilityId::NOTIFICATIONS, true);
    let dispatcher = notifications::NotificationDispatcher::new();

    // 1. Phone sends NotificationPost to Desktop
    let post = notifications::NotificationPost {
        notification_id: "notif-001".to_string(),
        package_name: "com.example.chat".to_string(),
        app_name: "ExampleChat".to_string(),
        title: "Alice".to_string(),
        body: "Hello from phone!".to_string(),
        timestamp: 1700000000,
        actions: vec![notifications::NotificationAction {
            action_id: "reply_act".to_string(),
            label: "Reply".to_string(),
            is_reply: true,
        }],
    };

    let ack = client_mux
        .send_notification_to_peer(&dispatcher, post.clone(), &query)
        .await
        .expect("send_notification_to_peer");
    assert!(ack.handled);

    let received_post = notif_rx.recv().await.expect("received post on desktop");
    assert_eq!(received_post.notification_id, "notif-001");
    assert_eq!(received_post.body, "Hello from phone!");

    // 2. Desktop sends quick reply action back to Phone
    let server_mux = server_handle.await.unwrap();
    let action_invoke = notifications::NotificationActionInvoke {
        notification_id: "notif-001".to_string(),
        action_id: "reply_act".to_string(),
        reply_text: "Replying from desktop keyboard!".to_string(),
    };

    let ack_action = server_mux
        .send_notification_action_to_peer(&dispatcher, action_invoke.clone(), &query)
        .await
        .expect("send_notification_action_to_peer");
    assert!(ack_action.handled);

    let received_action = action_rx.recv().await.expect("received action on phone");
    assert_eq!(received_action.notification_id, "notif-001");
    assert_eq!(
        received_action.reply_text,
        "Replying from desktop keyboard!"
    );

    // 3. Phone sends Dismiss to Desktop
    let dismiss = notifications::NotificationDismiss {
        notification_id: "notif-001".to_string(),
        package_name: "com.example.chat".to_string(),
    };

    let ack_dismiss = client_mux
        .send_notification_dismiss_to_peer(&dispatcher, dismiss.clone(), &query)
        .await
        .expect("send_notification_dismiss_to_peer");
    assert!(ack_dismiss.handled);

    let received_dismiss = dismiss_rx
        .recv()
        .await
        .expect("received dismiss on desktop");
    assert_eq!(received_dismiss.notification_id, "notif-001");
}
