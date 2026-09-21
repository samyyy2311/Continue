// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use bytes::BytesMut;
use limits::MAX_FRAME_CLIPBOARD_BYTES;
use crate::error::ClipboardError;

pub async fn write_msg<M: prost::Message>(
    stream: &mut quinn::SendStream,
    msg: &M,
) -> Result<(), ClipboardError> {
    let frame = protocol::encode_frame(msg, MAX_FRAME_CLIPBOARD_BYTES)?;
    stream
        .write_all(&frame)
        .await
        .map_err(transport::TransportError::from)?;
    Ok(())
}

pub async fn read_msg<M: prost::Message + Default>(
    stream: &mut quinn::RecvStream,
) -> Result<M, ClipboardError> {
    let mut buf = BytesMut::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(msg) = protocol::decode_frame_from_buf::<M>(&mut buf, MAX_FRAME_CLIPBOARD_BYTES)? {
            return Ok(msg);
        }
        match stream.read(&mut chunk).await.map_err(transport::TransportError::from)? {
            Some(n) if n > 0 => buf.extend_from_slice(&chunk[..n]),
            _ => return Err(transport::TransportError::ConnectionClosed.into()),
        }
    }
}
