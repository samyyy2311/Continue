// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! A file whose connection dropped part way picks up where it stopped the next time it is
//! sent, rather than starting over.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use protocol::v1::FileTransferRequest;
use transfer::{compute_file_sha256, receive_file, send_file, ReceivedFile, TransferError};
use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

const SIZE: usize = 8 * 1024 * 1024;

struct Pair {
    server: quinn::Endpoint,
    client: quinn::Endpoint,
    addr: SocketAddr,
}

fn pair() -> Pair {
    let server_cert = TransportCertificate::generate().unwrap();
    let client_cert = TransportCertificate::generate().unwrap();
    let server_tls = server_cert
        .build_pinned_server_tls(client_cert.spki_hash)
        .unwrap();
    let client_tls = client_cert
        .build_pinned_client_tls(server_cert.spki_hash)
        .unwrap();
    let any: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = create_server_endpoint(any, server_tls).unwrap();
    let addr = server.local_addr().unwrap();
    Pair {
        server,
        client: create_client_endpoint(any, client_tls).unwrap(),
        addr,
    }
}

/// What one attempt saw: the result on each side, and where each side started.
struct Attempt {
    sent: Result<u64, TransferError>,
    received: Result<ReceivedFile, TransferError>,
    sender_started_at: Option<u64>,
    receiver_started_at: Option<u64>,
}

/// Sends `file` over a new connection; the sender cuts the connection once `cut_after` bytes
/// have gone, if given.
async fn attempt(pair: &Pair, file: &Path, into: &Path, cut_after: Option<u64>) -> Attempt {
    let (server, into) = (pair.server.clone(), into.to_path_buf());
    let receiver_started_at = Arc::new(Mutex::new(None));
    let first = receiver_started_at.clone();
    let receiving = tokio::spawn(async move {
        let conn = server.accept().await.unwrap().await.unwrap();
        let (mut send, mut recv) = conn.accept_bi().await.unwrap();
        let received = receive_file(
            &mut send,
            &mut recv,
            &into,
            None::<fn(&_) -> std::future::Ready<bool>>,
            Some(move |_: &FileTransferRequest, at: u64| {
                first.lock().unwrap().get_or_insert(at);
            }),
            std::future::pending(),
        )
        .await;
        (received, conn)
    });

    let conn = pair
        .client
        .connect(pair.addr, "continue-device")
        .unwrap()
        .await
        .unwrap();
    let (mut send, mut recv) = conn.open_bi().await.unwrap();
    let sender_started_at = Arc::new(Mutex::new(None));
    let (first, cut) = (sender_started_at.clone(), conn.clone());
    let sent = send_file(
        &mut send,
        &mut recv,
        file,
        format!("tx-{}", rand::random::<u32>()),
        Some(move |done: u64, _total: u64| {
            first.lock().unwrap().get_or_insert(done);
            if cut_after.is_some_and(|limit| done >= limit) {
                cut.close(0u32.into(), b"wifi dropped");
            }
        }),
    )
    .await;

    let (received, _conn) = receiving.await.unwrap();
    let sender_started_at = *sender_started_at.lock().unwrap();
    let receiver_started_at = *receiver_started_at.lock().unwrap();
    Attempt {
        sent,
        received,
        sender_started_at,
        receiver_started_at,
    }
}

fn scratch() -> (PathBuf, PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("continue_resume_{}", rand::random::<u32>()));
    let (from, into) = (dir.join("sender"), dir.join("receiver"));
    std::fs::create_dir_all(&from).unwrap();
    std::fs::create_dir_all(&into).unwrap();
    (dir, from, into)
}

/// Different bytes throughout, so a resume from the wrong place can't pass the checksum.
fn video(from: &Path) -> (PathBuf, Vec<u8>) {
    let bytes: Vec<u8> = (0..SIZE).map(|i| (i % 251) as u8).collect();
    let path = from.join("holiday.mp4");
    std::fs::write(&path, &bytes).unwrap();
    (path, bytes)
}

fn leftovers(into: &Path) -> Vec<String> {
    std::fs::read_dir(into)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".part"))
        .collect()
}

#[tokio::test]
async fn a_dropped_transfer_resumes_where_it_stopped() {
    let (dir, from, into) = scratch();
    let (file, bytes) = video(&from);
    let pair = pair();

    let dropped = attempt(&pair, &file, &into, Some(SIZE as u64 / 2)).await;
    assert!(dropped.sent.is_err() && dropped.received.is_err());
    assert_eq!(leftovers(&into).len(), 1, "the partial file is kept");

    let resumed = attempt(&pair, &file, &into, None).await;
    let received = resumed.received.expect("the second attempt finishes");
    assert_eq!(resumed.sent.unwrap(), SIZE as u64);
    let started = resumed.receiver_started_at.unwrap();
    assert!(started > 0, "picked up part way, not from the start");
    assert_eq!(resumed.sender_started_at, Some(started), "both sides agree");
    assert_eq!(std::fs::read(&received.path).unwrap(), bytes);
    assert!(leftovers(&into).is_empty());

    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn a_damaged_partial_file_is_dropped_and_the_next_attempt_starts_over() {
    let (dir, from, into) = scratch();
    let (file, bytes) = video(&from);
    let pair = pair();

    // Half the file, but not the right half.
    let checksum = compute_file_sha256(&file).await.unwrap();
    let partial = into.join(format!(".continue-{}-{SIZE}.part", hex::encode(checksum)));
    std::fs::write(&partial, vec![0u8; SIZE / 2]).unwrap();

    let failed = attempt(&pair, &file, &into, None).await;
    assert_eq!(failed.receiver_started_at, Some(SIZE as u64 / 2));
    assert!(matches!(
        failed.received,
        Err(TransferError::ChecksumMismatch { .. })
    ));
    assert!(
        failed.sent.is_err(),
        "the sender hears it didn't arrive intact"
    );
    assert!(leftovers(&into).is_empty(), "the damaged partial is gone");

    let fresh = attempt(&pair, &file, &into, None).await;
    assert_eq!(fresh.receiver_started_at, Some(0));
    assert_eq!(std::fs::read(fresh.received.unwrap().path).unwrap(), bytes);

    let _ = std::fs::remove_dir_all(dir);
}
