// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::sync::Arc;
use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};
use transport::verifier::{PinnedClientCertVerifier, PinnedServerCertVerifier};

use transfer::{receive_file, send_file};

#[tokio::test]
async fn file_transfer_e2e_success() {
    let temp_dir = std::env::temp_dir().join(format!("continue_test_{}", rand::random::<u32>()));
    let send_dir = temp_dir.join("sender");
    let recv_dir = temp_dir.join("receiver");
    tokio::fs::create_dir_all(&send_dir).await.unwrap();
    tokio::fs::create_dir_all(&recv_dir).await.unwrap();

    // Create 150 KB test file (spans multiple 64 KB chunks)
    let test_file = send_dir.join("sample_document.pdf");
    let test_data = vec![0xABu8; 150 * 1024];
    tokio::fs::write(&test_file, &test_data).await.unwrap();

    // Setup TLS certificates
    let server_cert = TransportCertificate::generate().unwrap();
    let client_cert = TransportCertificate::generate().unwrap();

    let server_tls = server_cert.build_pinned_server_tls(client_cert.spki_hash).unwrap();
    let client_tls = client_cert.build_pinned_client_tls(server_cert.spki_hash).unwrap();

    let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server_endpoint = create_server_endpoint(server_addr, server_tls).unwrap();
    let bound_addr = server_endpoint.local_addr().unwrap();

    let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let client_endpoint = create_client_endpoint(client_addr, client_tls).unwrap();

    // Server receiver task
    let recv_dir_clone = recv_dir.clone();
    let recv_handle = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.expect("incoming conn");
        let conn = incoming.await.expect("established conn");
        let (mut send_stream, mut recv_stream) = conn.accept_bi().await.expect("accept bi stream");

        receive_file(
            &mut send_stream,
            &mut recv_stream,
            &recv_dir_clone,
            None::<fn(&_) -> bool>,
            None::<fn(u64, u64)>,
        )
        .await
    });

    // Client sender task
    let client_conn = client_endpoint.connect(bound_addr, "continue-device").unwrap().await.unwrap();
    let (mut client_send, mut client_recv) = client_conn.open_bi().await.unwrap();

    let bytes_sent = send_file(
        &mut client_send,
        &mut client_recv,
        &test_file,
        "tx-001".to_string(),
        None::<fn(u64, u64)>,
    )
    .await
    .unwrap();

    assert_eq!(bytes_sent, 150 * 1024);

    let recv_result = recv_handle.await.unwrap().expect("receive_file should succeed");
    assert_eq!(recv_result.bytes_received, 150 * 1024);
    assert_eq!(recv_result.file_name, "sample_document.pdf");

    // Verify written file contents on disk
    let written = tokio::fs::read(&recv_result.path).await.unwrap();
    assert_eq!(written, test_data);

    // Clean up
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
}

#[tokio::test]
async fn file_transfer_rejected_by_permission_checker() {
    let temp_dir = std::env::temp_dir().join(format!("continue_test_{}", rand::random::<u32>()));
    let send_dir = temp_dir.join("sender");
    let recv_dir = temp_dir.join("receiver");
    tokio::fs::create_dir_all(&send_dir).await.unwrap();
    tokio::fs::create_dir_all(&recv_dir).await.unwrap();

    let test_file = send_dir.join("secret.txt");
    tokio::fs::write(&test_file, b"classified").await.unwrap();

    let server_cert = TransportCertificate::generate().unwrap();
    let client_cert = TransportCertificate::generate().unwrap();

    let server_tls = server_cert.build_pinned_server_tls(client_cert.spki_hash).unwrap();
    let client_tls = client_cert.build_pinned_client_tls(server_cert.spki_hash).unwrap();

    let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server_endpoint = create_server_endpoint(server_addr, server_tls).unwrap();
    let bound_addr = server_endpoint.local_addr().unwrap();

    let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let client_endpoint = create_client_endpoint(client_addr, client_tls).unwrap();

    let recv_dir_clone = recv_dir.clone();
    let recv_handle = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.expect("incoming conn");
        let conn = incoming.await.expect("established conn");
        let (mut send_stream, mut recv_stream) = conn.accept_bi().await.expect("accept bi stream");

        // Permission checker rejects all transfers
        receive_file(
            &mut send_stream,
            &mut recv_stream,
            &recv_dir_clone,
            Some(|_req: &protocol::v1::FileTransferRequest| false),
            None::<fn(u64, u64)>,
        )
        .await
    });

    let client_conn = client_endpoint.connect(bound_addr, "continue-device").unwrap().await.unwrap();
    let (mut client_send, mut client_recv) = client_conn.open_bi().await.unwrap();

    let send_result = send_file(
        &mut client_send,
        &mut client_recv,
        &test_file,
        "tx-002".to_string(),
        None::<fn(u64, u64)>,
    )
    .await;

    assert!(matches!(send_result, Err(transfer::TransferError::Rejected(_))));
    assert!(recv_handle.await.unwrap().is_err());

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
}
