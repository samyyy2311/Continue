// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use sha2::{Digest, Sha256};
use std::future::Future;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use limits::{MAX_FRAME_MEDIA_CAPTURE_BYTES, TRANSFER_CHUNK_BYTES};
use protocol::v1::{
    CameraCaptureAck, CameraCaptureDestination, CameraCaptureMode, CameraCaptureRequest,
    CameraCaptureResponse, CameraCaptureStatus,
};
use transport::{read_msg, write_msg};

use crate::error::TransferError;
use crate::sanitizer::sanitize_filename;
use crate::sender::compute_file_sha256;

const BITMAPV5_HEADER_SIZE: usize = 124;
const BI_BITFIELDS: u32 = 3;
const LCS_SRGB: u32 = 0x7352_4742;
const LCS_GM_IMAGES: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedMedia {
    pub capture_id: String,
    pub mode: CameraCaptureMode,
    pub destination: CameraCaptureDestination,
    pub file_name: String,
    pub path: PathBuf,
    pub mime_type: String,
    pub bytes_received: u64,
    pub sha256_checksum: [u8; 32],
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct CameraCaptureSource {
    pub file_path: PathBuf,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
}

/// Formats raw BGRA uncompressed pixel buffers into the Windows CF_DIBV5 layout.
pub fn format_as_clipboard_dibv5(
    width: u32,
    height: u32,
    bgra_pixels: &[u8],
) -> Result<Vec<u8>, TransferError> {
    let expected_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| {
            TransferError::InvalidFilename("Dimensions exceed addressable bounds".to_string())
        })?;

    if bgra_pixels.len() != expected_len {
        return Err(TransferError::SizeMismatch {
            expected: expected_len as u64,
            actual: bgra_pixels.len() as u64,
        });
    }

    let mut out = Vec::with_capacity(BITMAPV5_HEADER_SIZE + expected_len);

    out.extend_from_slice(&(BITMAPV5_HEADER_SIZE as u32).to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    // Negative height signals a top-down DIB so rows need no vertical flip.
    out.extend_from_slice(&(-(height as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&BI_BITFIELDS.to_le_bytes());
    out.extend_from_slice(&(expected_len as u32).to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());

    // Color channel masks
    out.extend_from_slice(&0x00FF_0000u32.to_le_bytes());
    out.extend_from_slice(&0x0000_FF00u32.to_le_bytes());
    out.extend_from_slice(&0x0000_00FFu32.to_le_bytes());
    out.extend_from_slice(&0xFF00_0000u32.to_le_bytes());

    // Color space and endpoints
    out.extend_from_slice(&LCS_SRGB.to_le_bytes());
    out.extend_from_slice(&[0u8; 36]); // CIEXYZTRIPLE endpoints
    out.extend_from_slice(&0u32.to_le_bytes()); // Gamma red
    out.extend_from_slice(&0u32.to_le_bytes()); // Gamma green
    out.extend_from_slice(&0u32.to_le_bytes()); // Gamma blue
    out.extend_from_slice(&LCS_GM_IMAGES.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());

    debug_assert_eq!(out.len(), BITMAPV5_HEADER_SIZE);
    out.extend_from_slice(bgra_pixels);

    Ok(out)
}

pub async fn request_camera_capture(
    send_stream: &mut quinn::SendStream,
    recv_stream: &mut quinn::RecvStream,
    destination_dir: &Path,
    request: CameraCaptureRequest,
) -> Result<CapturedMedia, TransferError> {
    write_msg(send_stream, &request, MAX_FRAME_MEDIA_CAPTURE_BYTES).await?;

    let mut carry = bytes::BytesMut::with_capacity(TRANSFER_CHUNK_BYTES);
    let resp: CameraCaptureResponse = loop {
        if let Some(msg) = protocol::decode_frame_from_buf::<CameraCaptureResponse>(
            &mut carry,
            MAX_FRAME_MEDIA_CAPTURE_BYTES,
        )? {
            break msg;
        }
        let mut chunk = [0u8; 4096];
        match recv_stream.read(&mut chunk).await? {
            Some(n) if n > 0 => carry.extend_from_slice(&chunk[..n]),
            _ => return Err(TransferError::UnexpectedResponse),
        }
    };

    match CameraCaptureStatus::try_from(resp.status) {
        Ok(CameraCaptureStatus::Success) => {}
        Ok(CameraCaptureStatus::Declined) => {
            return Err(TransferError::CaptureRejected(resp.reason));
        }
        Ok(CameraCaptureStatus::Busy) => return Err(TransferError::CaptureBusy),
        Ok(CameraCaptureStatus::Unsupported) => return Err(TransferError::CaptureUnsupported),
        Ok(CameraCaptureStatus::Error) => return Err(TransferError::CaptureRejected(resp.reason)),
        _ => return Err(TransferError::UnexpectedResponse),
    }

    let default_ext = if resp.mime_type.contains("pdf") {
        "pdf"
    } else {
        "jpg"
    };

    let raw_name = if resp.file_name.is_empty() {
        format!("{}_{}.{}", match CameraCaptureMode::try_from(request.mode) {
            Ok(CameraCaptureMode::DocumentScan) => "scan",
            _ => "photo",
        }, resp.capture_id, default_ext)
    } else {
        resp.file_name
    };

    let file_name = sanitize_filename(&raw_name)?;
    tokio::fs::create_dir_all(destination_dir).await?;

    let final_path = destination_dir.join(&file_name);
    let part_path = destination_dir.join(format!("{}.part", file_name));

    let mut part_file = tokio::fs::File::create(&part_path).await?;
    let mut hasher = Sha256::new();
    let mut total_received = 0u64;

    if !carry.is_empty() {
        let initial_chunk = carry.freeze();
        hasher.update(&initial_chunk);
        part_file.write_all(&initial_chunk).await?;
        total_received += initial_chunk.len() as u64;
    }

    while total_received < resp.file_size {
        let to_read = std::cmp::min(
            (resp.file_size - total_received) as usize,
            TRANSFER_CHUNK_BYTES,
        );
        let mut buf = vec![0u8; to_read];
        let n = match recv_stream.read(&mut buf).await? {
            Some(n) if n > 0 => n,
            _ => {
                let _ = tokio::fs::remove_file(&part_path).await;
                return Err(TransferError::SizeMismatch {
                    expected: resp.file_size,
                    actual: total_received,
                });
            }
        };

        hasher.update(&buf[..n]);
        part_file.write_all(&buf[..n]).await?;
        total_received += n as u64;
    }

    part_file.flush().await?;
    drop(part_file);

    let actual_sha: [u8; 32] = hasher.finalize().into();
    if actual_sha.as_slice() != resp.sha256_checksum.as_slice() {
        let _ = tokio::fs::remove_file(&part_path).await;
        return Err(TransferError::ChecksumMismatch {
            expected: hex::encode(&resp.sha256_checksum),
            actual: hex::encode(actual_sha),
        });
    }

    tokio::fs::rename(&part_path, &final_path).await?;

    let ack = CameraCaptureAck {
        capture_id: resp.capture_id.clone(),
        bytes_received: total_received,
        verified: true,
    };
    write_msg(send_stream, &ack, MAX_FRAME_MEDIA_CAPTURE_BYTES).await?;
    send_stream.flush().await?;

    let mode = CameraCaptureMode::try_from(request.mode).unwrap_or(CameraCaptureMode::Unspecified);
    let destination = CameraCaptureDestination::try_from(request.destination)
        .unwrap_or(CameraCaptureDestination::Unspecified);

    Ok(CapturedMedia {
        capture_id: resp.capture_id,
        mode,
        destination,
        file_name,
        path: final_path,
        mime_type: resp.mime_type,
        bytes_received: total_received,
        sha256_checksum: actual_sha,
        width: resp.width,
        height: resp.height,
    })
}

pub async fn respond_camera_capture<P, Fut>(
    send_stream: &mut quinn::SendStream,
    recv_stream: &mut quinn::RecvStream,
    provider: P,
) -> Result<(), TransferError>
where
    P: FnOnce(CameraCaptureRequest) -> Fut,
    Fut: Future<Output = Result<CameraCaptureSource, TransferError>>,
{
    let req: CameraCaptureRequest =
        read_msg(recv_stream, MAX_FRAME_MEDIA_CAPTURE_BYTES).await?;

    let source = match provider(req.clone()).await {
        Ok(source) => source,
        Err(e) => {
            let (status, reason) = match &e {
                TransferError::CaptureBusy => (CameraCaptureStatus::Busy, "Camera is busy".into()),
                TransferError::CaptureUnsupported => {
                    (CameraCaptureStatus::Unsupported, "Camera unavailable".into())
                }
                TransferError::CaptureRejected(r) => (CameraCaptureStatus::Declined, r.clone()),
                other => (CameraCaptureStatus::Error, other.to_string()),
            };
            let resp = CameraCaptureResponse {
                capture_id: req.capture_id,
                status: status as i32,
                reason,
                mime_type: String::new(),
                file_size: 0,
                sha256_checksum: Vec::new(),
                width: 0,
                height: 0,
                file_name: String::new(),
            };
            write_msg(send_stream, &resp, MAX_FRAME_MEDIA_CAPTURE_BYTES).await?;
            return Err(e);
        }
    };

    let mut file = tokio::fs::File::open(&source.file_path).await?;
    let meta = file.metadata().await?;
    let file_size = meta.len();
    let sha256_bytes = compute_file_sha256(&source.file_path).await?;

    let file_name = source
        .file_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_default();

    let resp = CameraCaptureResponse {
        capture_id: req.capture_id.clone(),
        status: CameraCaptureStatus::Success as i32,
        reason: String::new(),
        mime_type: source.mime_type,
        file_size,
        sha256_checksum: sha256_bytes.to_vec(),
        width: source.width,
        height: source.height,
        file_name,
    };
    write_msg(send_stream, &resp, MAX_FRAME_MEDIA_CAPTURE_BYTES).await?;

    let mut buf = vec![0u8; TRANSFER_CHUNK_BYTES];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        send_stream.write_all(&buf[..n]).await?;
    }
    send_stream.flush().await?;

    let ack: CameraCaptureAck = read_msg(recv_stream, MAX_FRAME_MEDIA_CAPTURE_BYTES).await?;
    if !ack.verified {
        return Err(TransferError::UnexpectedResponse);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_dibv5_validates_dimensions_and_packs_header() {
        let width = 2;
        let height = 2;
        let pixels = vec![255u8; 16]; // 2 * 2 * 4 bytes

        let dib = format_as_clipboard_dibv5(width, height, &pixels).unwrap();
        assert_eq!(dib.len(), BITMAPV5_HEADER_SIZE + 16);

        // Check header size field
        let header_size = u32::from_le_bytes(dib[0..4].try_into().unwrap());
        assert_eq!(header_size, BITMAPV5_HEADER_SIZE as u32);

        // Check width and top-down height
        let w = i32::from_le_bytes(dib[4..8].try_into().unwrap());
        let h = i32::from_le_bytes(dib[8..12].try_into().unwrap());
        assert_eq!(w, 2);
        assert_eq!(h, -2);
    }

    #[test]
    fn format_dibv5_rejects_mismatched_pixel_buffer() {
        let width = 2;
        let height = 2;
        let pixels = vec![0u8; 12]; // Needs 16 bytes

        let err = format_as_clipboard_dibv5(width, height, &pixels).unwrap_err();
        assert!(matches!(err, TransferError::SizeMismatch { .. }));
    }
}
