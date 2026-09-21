// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use bytes::{Buf, BufMut, Bytes, BytesMut};
use prost::Message;

use crate::error::FrameError;

/// Length prefix header size in bytes (32-bit unsigned big-endian integer).
pub const FRAME_HEADER_LEN: usize = 4;

/// Encode a Protobuf message into a length-delimited frame.
///
/// Fails if the encoded payload exceeds `max_allowed_bytes`.
pub fn encode_frame<M: Message>(msg: &M, max_allowed_bytes: usize) -> Result<Bytes, FrameError> {
    let payload_len = msg.encoded_len();
    if payload_len > max_allowed_bytes {
        return Err(FrameError::FrameTooLarge {
            size: payload_len,
            limit: max_allowed_bytes,
        });
    }

    let mut buf = BytesMut::with_capacity(FRAME_HEADER_LEN + payload_len);
    buf.put_u32(payload_len as u32);
    msg.encode(&mut buf)?;
    Ok(buf.freeze())
}

/// Encode raw pre-serialized bytes into a length-delimited frame.
pub fn encode_raw_frame(payload: &[u8], max_allowed_bytes: usize) -> Result<Bytes, FrameError> {
    if payload.len() > max_allowed_bytes {
        return Err(FrameError::FrameTooLarge {
            size: payload.len(),
            limit: max_allowed_bytes,
        });
    }

    let mut buf = BytesMut::with_capacity(FRAME_HEADER_LEN + payload.len());
    buf.put_u32(payload.len() as u32);
    buf.put_slice(payload);
    Ok(buf.freeze())
}

/// Decode a Protobuf message from a complete length-delimited frame slice.
pub fn decode_frame<M: Message + Default>(
    src: &[u8],
    max_allowed_bytes: usize,
) -> Result<M, FrameError> {
    if src.len() < FRAME_HEADER_LEN {
        return Err(FrameError::Incomplete {
            needed: FRAME_HEADER_LEN,
            available: src.len(),
        });
    }

    let payload_len = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;
    if payload_len > max_allowed_bytes {
        return Err(FrameError::FrameTooLarge {
            size: payload_len,
            limit: max_allowed_bytes,
        });
    }

    let total_len = FRAME_HEADER_LEN + payload_len;
    if src.len() < total_len {
        return Err(FrameError::Incomplete {
            needed: total_len,
            available: src.len(),
        });
    }

    let payload = &src[FRAME_HEADER_LEN..total_len];
    Ok(M::decode(payload)?)
}

/// Attempt to extract a single length-delimited frame from a streaming buffer.
///
/// Returns:
/// - `Ok(Some(msg))` when a complete valid frame has been extracted and consumed.
/// - `Ok(None)` if the buffer does not yet contain a complete frame.
/// - `Err(FrameError)` if the frame length exceeds `max_allowed_bytes` or payload decoding fails.
///
/// The length check is enforced before allocating or consuming payload data.
pub fn decode_frame_from_buf<M: Message + Default>(
    buf: &mut BytesMut,
    max_allowed_bytes: usize,
) -> Result<Option<M>, FrameError> {
    if let Some(raw_payload) = decode_raw_frame_from_buf(buf, max_allowed_bytes)? {
        let msg = M::decode(raw_payload)?;
        Ok(Some(msg))
    } else {
        Ok(None)
    }
}

/// Attempt to extract raw length-delimited frame bytes from a streaming buffer.
///
/// Bounded check: if the advertised length exceeds `max_allowed_bytes`, the error
/// is returned immediately without waiting for or allocating the frame bytes.
pub fn decode_raw_frame_from_buf(
    buf: &mut BytesMut,
    max_allowed_bytes: usize,
) -> Result<Option<Bytes>, FrameError> {
    if buf.len() < FRAME_HEADER_LEN {
        return Ok(None);
    }

    let payload_len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if payload_len > max_allowed_bytes {
        return Err(FrameError::FrameTooLarge {
            size: payload_len,
            limit: max_allowed_bytes,
        });
    }

    if buf.len() < FRAME_HEADER_LEN + payload_len {
        return Ok(None);
    }

    buf.advance(FRAME_HEADER_LEN);
    let payload = buf.split_to(payload_len).freeze();
    Ok(Some(payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;

    #[derive(Clone, PartialEq, Message, Default)]
    struct DummyMessage {
        #[prost(uint64, tag = "1")]
        seq: u64,
        #[prost(string, tag = "2")]
        data: String,
    }

    #[test]
    fn round_trip_frame_encode_decode() {
        let msg = DummyMessage {
            seq: 42,
            data: "hello world".to_string(),
        };

        let encoded = encode_frame(&msg, 1024).unwrap();
        assert_eq!(encoded.len(), FRAME_HEADER_LEN + msg.encoded_len());

        let decoded: DummyMessage = decode_frame(&encoded, 1024).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn encode_rejects_oversized_message() {
        let msg = DummyMessage {
            seq: 1,
            data: "x".repeat(100),
        };

        let result = encode_frame(&msg, 50);
        assert!(matches!(result, Err(FrameError::FrameTooLarge { .. })));
    }

    #[test]
    fn decode_rejects_oversized_frame_header_immediately() {
        let mut buf = BytesMut::new();
        // Claim payload is 100_000 bytes with limit of 1024.
        buf.put_u32(100_000);
        buf.put_slice(b"small prefix");

        let result = decode_raw_frame_from_buf(&mut buf, 1024);
        assert!(matches!(
            result,
            Err(FrameError::FrameTooLarge {
                size: 100_000,
                limit: 1024
            })
        ));
    }

    #[test]
    fn streaming_buffer_waits_for_complete_frame() {
        let msg = DummyMessage {
            seq: 99,
            data: "test".to_string(),
        };
        let encoded = encode_frame(&msg, 1024).unwrap();

        let mut buf = BytesMut::new();

        // Feed only 2 bytes (incomplete header)
        buf.put_slice(&encoded[..2]);
        assert_eq!(
            decode_frame_from_buf::<DummyMessage>(&mut buf, 1024).unwrap(),
            None
        );

        // Feed rest of header and 1 byte of payload
        buf.put_slice(&encoded[2..FRAME_HEADER_LEN + 1]);
        assert_eq!(
            decode_frame_from_buf::<DummyMessage>(&mut buf, 1024).unwrap(),
            None
        );

        // Feed remaining payload
        buf.put_slice(&encoded[FRAME_HEADER_LEN + 1..]);
        let decoded = decode_frame_from_buf::<DummyMessage>(&mut buf, 1024)
            .unwrap()
            .expect("should decode full message");
        assert_eq!(decoded, msg);
        assert!(buf.is_empty());
    }
}
