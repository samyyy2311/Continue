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

/// Reads one length-prefixed protobuf message, and nothing past it, so a stream can carry many.
/// The limit is checked against the declared length before the payload is buffered, so a peer
/// cannot force large allocations.
pub async fn read_msg<M: Message + Default>(
    stream: &mut quinn::RecvStream,
    max_bytes: usize,
) -> Result<M, TransportError> {
    let mut header = [0u8; protocol::FRAME_HEADER_LEN];
    stream.read_exact(&mut header).await?;
    let len = u32::from_be_bytes(header) as usize;
    if len > max_bytes {
        return Err(protocol::FrameError::FrameTooLarge {
            size: len,
            limit: max_bytes,
        }
        .into());
    }
    let mut frame = BytesMut::zeroed(protocol::FRAME_HEADER_LEN + len);
    frame[..protocol::FRAME_HEADER_LEN].copy_from_slice(&header);
    stream
        .read_exact(&mut frame[protocol::FRAME_HEADER_LEN..])
        .await?;
    Ok(protocol::decode_frame(&frame, max_bytes)?)
}
