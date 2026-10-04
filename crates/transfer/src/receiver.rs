// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use sha2::Digest;
use std::future::Future;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

use limits::{MAX_FRAME_TRANSFER_META_BYTES, TRANSFER_CHUNK_BYTES};
use protocol::v1::{
    FileTransferAck, FileTransferRequest, FileTransferResponse, TransferResponseStatus,
};
use transport::{read_msg, write_msg};

use crate::error::TransferError;
use crate::partial;
use crate::sanitizer::sanitize_filename;

/// Told to the sender when the receiver stops a transfer part way.
const CANCELLED_CODE: u32 = 1;

/// Result of a completed and verified incoming file transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedFile {
    pub transfer_id: String,
    pub path: PathBuf,
    pub file_name: String,
    pub bytes_received: u64,
}

/// Receive an incoming file transfer over a dedicated QUIC bidirectional stream.
///
/// `permission_checker` sees the request before anything is written and may take its time,
/// e.g. to ask the user; the sender waits for the answer. Once accepted, `on_progress` hears
/// how many bytes have arrived, starting with what an earlier, interrupted attempt at the
/// same file left (0 for a new one). When `stop` finishes first, the transfer is abandoned,
/// the partial file removed and the sender told. When the connection drops instead, a
/// resumable request's partial file is kept for the next attempt.
pub async fn receive_file<P, Fut, F, S>(
    send_stream: &mut quinn::SendStream,
    recv_stream: &mut quinn::RecvStream,
    destination_dir: &Path,
    permission_checker: Option<P>,
    on_progress: Option<F>,
    stop: S,
) -> Result<ReceivedFile, TransferError>
where
    P: FnOnce(&FileTransferRequest) -> Fut,
    Fut: Future<Output = bool>,
    F: Fn(&FileTransferRequest, u64),
    S: Future<Output = ()>,
{
    let req: FileTransferRequest = read_msg(recv_stream, MAX_FRAME_TRANSFER_META_BYTES).await?;

    let clean_name = match sanitize_filename(&req.file_name) {
        Ok(name) => name,
        Err(e) => {
            let resp = FileTransferResponse {
                transfer_id: req.transfer_id,
                status: TransferResponseStatus::Rejected as i32,
                reason: e.to_string(),
                resume_offset: 0,
            };
            write_msg(send_stream, &resp, MAX_FRAME_TRANSFER_META_BYTES).await?;
            return Err(e);
        }
    };

    if let Some(checker) = permission_checker {
        if !checker(&req).await {
            let resp = FileTransferResponse {
                transfer_id: req.transfer_id,
                status: TransferResponseStatus::Rejected as i32,
                reason: "Permission denied".to_string(),
                resume_offset: 0,
            };
            write_msg(send_stream, &resp, MAX_FRAME_TRANSFER_META_BYTES).await?;
            return Err(TransferError::Rejected("Permission denied".to_string()));
        }
    }

    tokio::fs::create_dir_all(destination_dir).await?;
    partial::prune_stale(destination_dir).await;

    // Isolate incoming payload in temporary file until checksum and length are verified.
    let part = partial::claim_for(destination_dir, &req);
    let part_path = part.path.clone();
    let (resume_offset, mut hasher) = partial::already_received(&part, req.file_size).await;
    let mut part_file = if resume_offset > 0 {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&part_path)
            .await?
    } else {
        tokio::fs::File::create(&part_path).await?
    };

    let resp = FileTransferResponse {
        transfer_id: req.transfer_id.clone(),
        status: TransferResponseStatus::Accepted as i32,
        reason: String::new(),
        resume_offset,
    };
    write_msg(send_stream, &resp, MAX_FRAME_TRANSFER_META_BYTES).await?;
    if let Some(ref progress) = on_progress {
        progress(&req, resume_offset);
    }

    let mut total_received = resume_offset;
    let mut chunk = vec![0u8; TRANSFER_CHUNK_BYTES];

    let copy = async {
        loop {
            match recv_stream.read(&mut chunk).await? {
                Some(n) if n > 0 => {
                    part_file.write_all(&chunk[..n]).await?;
                    hasher.update(&chunk[..n]);
                    total_received += n as u64;
                    if let Some(ref progress) = on_progress {
                        progress(&req, total_received);
                    }
                }
                _ => break,
            }
        }
        part_file.flush().await?;
        Ok(())
    };
    let stream_result: Result<(), TransferError> = tokio::select! {
        result = copy => result,
        () = stop => Err(TransferError::Cancelled),
    };
    if matches!(stream_result, Err(TransferError::Cancelled)) {
        let _ = recv_stream.stop(quinn::VarInt::from_u32(CANCELLED_CODE));
    }

    // Drop handle before renaming or deleting to release file lock on Windows.
    drop(part_file);

    if let Err(e) = stream_result {
        // Kept for a later attempt only when the connection, not the user, ended it.
        if !part.resumable || matches!(e, TransferError::Cancelled) {
            let _ = tokio::fs::remove_file(&part_path).await;
        }
        return Err(e);
    }

    if total_received != req.file_size {
        let _ = tokio::fs::remove_file(&part_path).await;
        let ack = FileTransferAck {
            transfer_id: req.transfer_id,
            bytes_received: total_received,
            verified: false,
        };
        let _ = write_msg(send_stream, &ack, MAX_FRAME_TRANSFER_META_BYTES).await;
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
        let _ = write_msg(send_stream, &ack, MAX_FRAME_TRANSFER_META_BYTES).await;
        return Err(TransferError::ChecksumMismatch {
            expected: hex::encode(&req.sha256_checksum),
            actual: hex::encode(actual_hash),
        });
    }

    let target_path = resolve_unique_path(destination_dir, &clean_name).await;
    tokio::fs::rename(&part_path, &target_path).await?;

    let ack = FileTransferAck {
        transfer_id: req.transfer_id.clone(),
        bytes_received: total_received,
        verified: true,
    };
    write_msg(send_stream, &ack, MAX_FRAME_TRANSFER_META_BYTES).await?;

    Ok(ReceivedFile {
        transfer_id: req.transfer_id,
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
    let ext = Path::new(file_name).extension().and_then(|e| e.to_str());

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
