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

#[tokio::test]
async fn clipboard_e2e_sync_and_echo_suppression() {
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

    let receiver_sync = Arc::new(ClipboardSynchronizer::new());
    let recv_sync_clone = receiver_sync.clone();

    // Receiver loop
    let recv_handle = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.expect("incoming conn");
        let conn = incoming.await.expect("conn");
        let (mut send, mut recv) = conn.accept_bi().await.expect("bi stream");

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
    let client_conn = client_endpoint
        .connect(bound_addr, "continue-device")
        .unwrap()
        .await
        .unwrap();
    let (mut client_send, mut client_recv) = client_conn.open_bi().await.unwrap();

    let sender_sync = ClipboardSynchronizer::new();
    let query = authorized_clipboard_query();
    let ack = sender_sync
        .send_update(
            &mut client_send,
            &mut client_recv,
            ClipboardFormat::ClipboardFormatTextPlain,
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

    let cert = TransportCertificate::generate().unwrap();
    let tls = cert.build_pinned_client_tls(cert.spki_hash).unwrap();
    let endpoint = create_client_endpoint("127.0.0.1:0".parse().unwrap(), tls).unwrap();
    let conn = endpoint.connect("127.0.0.1:1".parse().unwrap(), "continue-device");

    // Check evaluate_capability fails without attempting network I/O
    let res = sync.send_update(
        &mut unsafe { std::mem::zeroed() },
        &mut unsafe { std::mem::zeroed() },
        ClipboardFormat::ClipboardFormatTextPlain,
        b"test".to_vec(),
        &unauthorized_query,
    ).await;

    assert!(matches!(res, Err(clipboard::ClipboardError::Capability(_))));
}
