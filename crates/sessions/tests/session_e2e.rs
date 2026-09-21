// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use protocol::v1::{session_envelope::Body, DisconnectReason, Ping, Pong, SessionEnvelope};
use protocol::CapabilityId;
use sessions::{open_capability_stream, Session, SessionMultiplexer};
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

    // Server router task
    let server_handle = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.expect("incoming conn");
        let conn = incoming.await.expect("conn");
        let mux = SessionMultiplexer::new("client-fingerprint".to_string(), conn);
        let mut rx = mux.spawn_router(8);

        // Receive stream 1: File Transfer
        let stream1 = rx.recv().await.expect("receive stream 1");
        assert_eq!(stream1.capability, CapabilityId::FILE_TRANSFER);

        // Receive stream 2: Clipboard
        let stream2 = rx.recv().await.expect("receive stream 2");
        assert_eq!(stream2.capability, CapabilityId::CLIPBOARD);

        mux
    });

    // Client connection
    let client_conn = client_endpoint
        .connect(bound_addr, "continue-device")
        .unwrap()
        .await
        .unwrap();

    let client_mux = SessionMultiplexer::new("server-fingerprint".to_string(), client_conn);

    // Open file transfer stream
    let (mut send1, _recv1) = client_mux.open_stream(CapabilityId::FILE_TRANSFER).await.unwrap();
    send1.finish().unwrap();

    // Open clipboard stream
    let (mut send2, _recv2) = client_mux.open_stream(CapabilityId::CLIPBOARD).await.unwrap();
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
    let (mut send, mut recv) = open_capability_stream(&client_conn, CapabilityId::CONTROL).await.unwrap();

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
    server_mux.disconnect(DisconnectReason::DisconnectReasonNormal, "test complete".to_string()).await;
}
