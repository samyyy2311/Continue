// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use capabilities::CapabilityQuery;
use clipboard::{ClipboardFormat, ClipboardSynchronizer};
use protocol::CapabilityId;
use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

fn authorized_clipboard_query() -> CapabilityQuery {
    let mut caps = HashSet::new();
    caps.insert(CapabilityId::CLIPBOARD);
    CapabilityQuery {
        capability: CapabilityId::CLIPBOARD,
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
async fn clipboard_e2e_sync_and_echo_suppression() {
    let (client_conn, server_conn) = loopback_connections().await;

    let receiver_sync = Arc::new(ClipboardSynchronizer::new());
    let recv_sync_clone = receiver_sync.clone();

    // Receiver loop. It gets a clone so `server_conn` keeps the connection open
    // until the sender has read the reply.
    let receiver_conn = server_conn.clone();
    let recv_handle = tokio::spawn(async move {
        let (mut send, mut recv) = receiver_conn.accept_bi().await.expect("bi stream");

        let query = authorized_clipboard_query();
        let received = recv_sync_clone
            .receive_update(&mut send, &mut recv, &query, |_fmt, payload| {
                assert_eq!(payload, b"Hello from Continue clipboard sync!");
                Ok(())
            })
            .await
            .expect("receive update");

        assert_eq!(received.sequence_number, 1);
    });

    // Sender
    let (mut client_send, mut client_recv) = client_conn.open_bi().await.unwrap();

    let sender_sync = ClipboardSynchronizer::new();
    let query = authorized_clipboard_query();
    let ack = sender_sync
        .send_update(
            &mut client_send,
            &mut client_recv,
            ClipboardFormat::TextPlain,
            b"Hello from Continue clipboard sync!".to_vec(),
            &query,
        )
        .await
        .expect("send update");

    assert!(ack.applied);
    assert_eq!(ack.sequence_number, 1);

    recv_handle.await.unwrap();
}

#[tokio::test]
async fn clipboard_rejects_unauthorized_capability() {
    let sync = ClipboardSynchronizer::new();
    let unauthorized_query = CapabilityQuery {
        capability: CapabilityId::CLIPBOARD,
        is_os_available: true,
        is_app_permitted: false, // app permission denied
        is_peer_authorized: true,
        negotiated_session_capabilities: HashSet::new(),
    };

    let (client_conn, _server_conn) = loopback_connections().await;
    let (mut send, mut recv) = client_conn.open_bi().await.unwrap();

    // Rejected before anything is written to the stream.
    let res = sync
        .send_update(
            &mut send,
            &mut recv,
            ClipboardFormat::TextPlain,
            b"test".to_vec(),
            &unauthorized_query,
        )
        .await;

    assert!(matches!(res, Err(clipboard::ClipboardError::Capability(_))));
}
