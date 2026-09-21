// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use limits::TRANSFER_CHUNK_BYTES;
use protocol::v1::{FileTransferAck, FileTransferRequest, FileTransferResponse, TransferResponseStatus};

use crate::error::TransferError;
use crate::sanitizer::sanitize_filename;
use crate::wire::{hex_encode, read_msg, write_msg};

/// Result of a completed and verified incoming file transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedFile {
    pub path: PathBuf,
    pub file_name: String,
    pub bytes_received: u64,
}

/// Receive an incoming file transfer over a dedicated QUIC bidirectional stream.
pub async fn receive_file<P, F>(
    send_stream: &mut quinn::SendStream,
    recv_stream: &mut quinn::RecvStream,
    destination_dir: &Path,
    permission_checker: Option<P>,
    on_progress: Option<F>,
) -> Result<ReceivedFile, TransferError>
where
    P: Fn(&FileTransferRequest) -> bool,
    F: Fn(u64, u64),
{
    let req: FileTransferRequest = read_msg(recv_stream).await?;

    let clean_name = match sanitize_filename(&req.file_name) {
        Ok(name) => name,
        Err(e) => {
            let resp = FileTransferResponse {
                transfer_id: req.transfer_id,
                status: TransferResponseStatus::Rejected as i32,
                reason: e.to_string(),
            };
            write_msg(send_stream, &resp).await?;
            return Err(e);
        }
    };

    if let Some(ref checker) = permission_checker {
        if !checker(&req) {
            let resp = FileTransferResponse {
                transfer_id: req.transfer_id,
                status: TransferResponseStatus::Rejected as i32,
                reason: "Permission denied".to_string(),
            };
            write_msg(send_stream, &resp).await?;
            return Err(TransferError::Rejected("Permission denied".to_string()));
        }
    }

    tokio::fs::create_dir_all(destination_dir).await?;

    // Isolate incoming payload in temporary file until checksum and length are verified.
    let part_path = destination_dir.join(format!("{}.continue_part", req.transfer_id));
    let mut part_file = tokio::fs::File::create(&part_path).await?;

    let resp = FileTransferResponse {
        transfer_id: req.transfer_id.clone(),
        status: TransferResponseStatus::Accepted as i32,
        reason: String::new(),
    };
    write_msg(send_stream, &resp).await?;

    let mut hasher = Sha256::new();
    let mut total_received = 0u64;
    let mut chunk = vec![0u8; TRANSFER_CHUNK_BYTES];

    let stream_result: Result<(), TransferError> = async {
        loop {
            match recv_stream.read(&mut chunk).await? {
                Some(n) if n > 0 => {
                    part_file.write_all(&chunk[..n]).await?;
                    hasher.update(&chunk[..n]);
                    total_received += n as u64;
                    if let Some(ref progress) = on_progress {
                        progress(total_received, req.file_size);
                    }
                }
                _ => break,
            }
        }
        part_file.flush().await?;
        Ok(())
    }
    .await;

    // Drop handle before renaming or deleting to release file lock on Windows.
    drop(part_file);

    if let Err(e) = stream_result {
        let _ = tokio::fs::remove_file(&part_path).await;
        return Err(e);
    }

    if total_received != req.file_size {
        let _ = tokio::fs::remove_file(&part_path).await;
        let ack = FileTransferAck {
            transfer_id: req.transfer_id,
            bytes_received: total_received,
            verified: false,
        };
        let _ = write_msg(send_stream, &ack).await;
        return Err(TransferError::SizeMismatch {
            expected: req.file_size,
            actual: total_received,
        });
    }

    let actual_hash: [u8; 32] = hasher.finalize().into();
    if actual_hash.as_slice() != req.sha256_checksum.as_slice() {
        let _ = tokio::fs::remove_file(&part_path).await;
        let ack = FileTransferAck {
            transfer_id: req.transfer_id,
            bytes_received: total_received,
            verified: false,
        };
        let _ = write_msg(send_stream, &ack).await;
        return Err(TransferError::ChecksumMismatch {
            expected: hex_encode(&req.sha256_checksum),
            actual: hex_encode(actual_hash),
        });
    }

    let target_path = resolve_unique_path(destination_dir, &clean_name).await;
    tokio::fs::rename(&part_path, &target_path).await?;

    let ack = FileTransferAck {
        transfer_id: req.transfer_id,
        bytes_received: total_received,
        verified: true,
    };
    write_msg(send_stream, &ack).await?;

    Ok(ReceivedFile {
        path: target_path,
        file_name: clean_name,
        bytes_received: total_received,
    })
}

async fn resolve_unique_path(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }

    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name);
    let ext = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str());

    for i in 1..1000 {
        let name = match ext {
            Some(e) => format!("{stem} ({i}).{e}"),
            None => format!("{stem} ({i})"),
        };
        let path = dir.join(name);
        if !path.exists() {
            return path;
        }
    }

    candidate
}
