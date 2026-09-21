// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use limits::TRANSFER_CHUNK_BYTES;
use protocol::v1::{FileTransferAck, FileTransferRequest, FileTransferResponse, TransferResponseStatus};

use crate::error::TransferError;
use crate::sanitizer::sanitize_filename;
use crate::wire::{hex_encode, read_msg, write_msg};

/// Compute the SHA-256 digest of a file on disk.
pub async fn compute_file_sha256(path: &Path) -> Result<[u8; 32], TransferError> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; TRANSFER_CHUNK_BYTES];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().into())
}

/// Send a local file to a peer over a dedicated QUIC bidirectional stream.
pub async fn send_file<F>(
    send_stream: &mut quinn::SendStream,
    recv_stream: &mut quinn::RecvStream,
    file_path: &Path,
    transfer_id: String,
    on_progress: Option<F>,
) -> Result<u64, TransferError>
where
    F: Fn(u64, u64),
{
    let mut file = tokio::fs::File::open(file_path).await?;
    let meta = file.metadata().await?;
    let file_size = meta.len();

    let raw_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| TransferError::InvalidFilename("Invalid file path".to_string()))?;
    let file_name = sanitize_filename(raw_name)?;

    let sha256_bytes = compute_file_sha256(file_path).await?;

    let req = FileTransferRequest {
        transfer_id: transfer_id.clone(),
        file_name,
        file_size,
        sha256_checksum: sha256_bytes.to_vec(),
        mime_type: String::new(),
    };
    write_msg(send_stream, &req).await?;

    let resp: FileTransferResponse = read_msg(recv_stream).await?;
    if resp.transfer_id != transfer_id {
        return Err(TransferError::UnexpectedResponse);
    }
    match resp.status() {
        TransferResponseStatus::Accepted => {}
        TransferResponseStatus::Rejected => {
            return Err(TransferError::Rejected(resp.reason));
        }
        TransferResponseStatus::Busy => {
            return Err(TransferError::Busy);
        }
        _ => {
            return Err(TransferError::Rejected(resp.reason));
        }
    }

    let mut chunk = vec![0u8; TRANSFER_CHUNK_BYTES];
    let mut bytes_sent = 0u64;
    loop {
        let n = file.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        send_stream.write_all(&chunk[..n]).await?;
        bytes_sent += n as u64;
        if let Some(ref progress) = on_progress {
            progress(bytes_sent, file_size);
        }
    }

    send_stream.finish()?;

    let ack: FileTransferAck = read_msg(recv_stream).await?;
    if ack.transfer_id != transfer_id {
        return Err(TransferError::UnexpectedResponse);
    }
    if !ack.verified || ack.bytes_received != file_size {
        return Err(TransferError::ChecksumMismatch {
            expected: hex_encode(sha256_bytes),
            actual: format!(
                "received {} of {} bytes (verified={})",
                ack.bytes_received, file_size, ack.verified
            ),
        });
    }

    Ok(bytes_sent)
}
