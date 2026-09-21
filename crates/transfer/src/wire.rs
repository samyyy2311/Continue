// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use bytes::BytesMut;
use limits::MAX_FRAME_TRANSFER_META_BYTES;
use crate::error::TransferError;

pub async fn write_msg<M: prost::Message>(
    stream: &mut quinn::SendStream,
    msg: &M,
) -> Result<(), TransferError> {
    let frame = protocol::encode_frame(msg, MAX_FRAME_TRANSFER_META_BYTES)?;
    stream.write_all(&frame).await?;
    Ok(())
}

pub async fn read_msg<M: prost::Message + Default>(
    stream: &mut quinn::RecvStream,
) -> Result<M, TransferError> {
    let mut buf = BytesMut::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        if let Some(msg) = protocol::decode_frame_from_buf::<M>(&mut buf, MAX_FRAME_TRANSFER_META_BYTES)? {
            return Ok(msg);
        }
        match stream.read(&mut chunk).await? {
            Some(n) if n > 0 => buf.extend_from_slice(&chunk[..n]),
            _ => return Err(TransferError::UnexpectedResponse),
        }
    }
}

pub fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    let mut s = String::new();
    for b in bytes.as_ref() {
        use std::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}
