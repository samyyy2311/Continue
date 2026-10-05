// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use bytes::BytesMut;
use prost::Message;

use crate::TransportError;

/// Writes one length-prefixed protobuf message, refusing anything over `max_bytes`.
pub async fn write_msg<M: Message>(
    stream: &mut quinn::SendStream,
    msg: &M,
    max_bytes: usize,
) -> Result<(), TransportError> {
    let frame = protocol::encode_frame(msg, max_bytes)?;
    stream.write_all(&frame).await?;
    Ok(())
}

/// Reads one length-prefixed protobuf message. The limit is checked against
/// the declared length before the payload is buffered, so a peer cannot force large
/// allocations.
pub async fn read_msg<M: Message + Default>(
    stream: &mut quinn::RecvStream,
    max_bytes: usize,
) -> Result<M, TransportError> {
    let mut buf = BytesMut::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(msg) = protocol::decode_frame_from_buf::<M>(&mut buf, max_bytes)? {
            return Ok(msg);
        }
        match stream.read(&mut chunk).await? {
            Some(n) if n > 0 => buf.extend_from_slice(&chunk[..n]),
            _ => return Err(TransportError::ConnectionClosed),
        }
    }
}

/// Reads raw frame bytes of one length-prefixed frame without decoding into a specific type.
pub async fn read_raw_msg(
    stream: &mut quinn::RecvStream,
    max_bytes: usize,
) -> Result<bytes::Bytes, TransportError> {
    let mut buf = BytesMut::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(raw) = protocol::decode_raw_frame_from_buf(&mut buf, max_bytes)? {
            return Ok(raw);
        }
        match stream.read(&mut chunk).await? {
            Some(n) if n > 0 => buf.extend_from_slice(&chunk[..n]),
            _ => return Err(TransportError::ConnectionClosed),
        }
    }
}
