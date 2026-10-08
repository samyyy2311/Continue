// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::{MAX_FRAME_REMOTE_INPUT_BYTES, MAX_REMOTE_INPUT_TEXT_BYTES};
use protocol::v1::{
    remote_input_envelope::Payload, RemoteInputAck, RemoteInputEnvelope, TextInputChunk,
};
use transport::{read_msg, write_msg};

use crate::error::SessionError;

#[derive(Default, Clone)]
pub struct RemoteInputDispatcher;

impl RemoteInputDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_chunk(chunk: &TextInputChunk) -> Result<(), SessionError> {
        if chunk.text.len() > MAX_REMOTE_INPUT_TEXT_BYTES {
            return Err(SessionError::RemoteInputValidation(format!(
                "Text chunk length {} exceeds maximum allowed of {} bytes",
                chunk.text.len(),
                MAX_REMOTE_INPUT_TEXT_BYTES
            )));
        }
        Ok(())
    }

    /// Sends a text chunk (dictation or keyboard input) to the remote peer.
    pub async fn send_chunk(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        chunk: TextInputChunk,
        query: &CapabilityQuery,
    ) -> Result<RemoteInputAck, SessionError> {
        evaluate_capability(query)?;
        Self::validate_chunk(&chunk)?;

        let envelope = RemoteInputEnvelope {
            payload: Some(Payload::Chunk(chunk)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_REMOTE_INPUT_BYTES).await?;
        let ack: RemoteInputAck = read_msg(recv_stream, MAX_FRAME_REMOTE_INPUT_BYTES).await?;

        Ok(ack)
    }

    /// Receives a text input envelope, passes the chunk to the handler, and writes back an acknowledgment.
    pub async fn receive_envelope<H>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        mut on_chunk: H,
    ) -> Result<TextInputChunk, SessionError>
    where
        H: FnMut(TextInputChunk) -> RemoteInputAck,
    {
        evaluate_capability(query)?;

        let envelope: RemoteInputEnvelope =
            read_msg(recv_stream, MAX_FRAME_REMOTE_INPUT_BYTES).await?;

        match envelope.payload {
            Some(Payload::Chunk(chunk)) => {
                Self::validate_chunk(&chunk)?;
                let ack = on_chunk(chunk.clone());
                write_msg(send_stream, &ack, MAX_FRAME_REMOTE_INPUT_BYTES).await?;
                Ok(chunk)
            }
            Some(Payload::Ack(_)) | None => Err(SessionError::UnexpectedMessage),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::CapabilityId;
    use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

    #[tokio::test]
    async fn remote_input_roundtrip() {
        let server_cert = TransportCertificate::generate().unwrap();
        let client_cert = TransportCertificate::generate().unwrap();

        let server_tls = server_cert
            .build_pinned_server_tls(client_cert.spki_hash)
            .unwrap();
        let client_tls = client_cert
            .build_pinned_client_tls(server_cert.spki_hash)
            .unwrap();

        let server_endpoint =
            create_server_endpoint("127.0.0.1:0".parse().unwrap(), server_tls).unwrap();
        let bound_addr = server_endpoint.local_addr().unwrap();

        let client_endpoint =
            create_client_endpoint("127.0.0.1:0".parse().unwrap(), client_tls).unwrap();

        let server_task = tokio::spawn(async move {
            let incoming = server_endpoint.accept().await.unwrap();
            let conn = incoming.await.unwrap();
            let (mut send_stream, mut recv_stream) = conn.accept_bi().await.unwrap();

            let dispatcher = RemoteInputDispatcher::new();
            let query = CapabilityQuery::negotiated(CapabilityId::REMOTE_INPUT, true);

            let chunk = dispatcher
                .receive_envelope(&mut send_stream, &mut recv_stream, &query, |chunk| {
                    RemoteInputAck {
                        success: chunk.commit,
                        error_message: String::new(),
                    }
                })
                .await;

            (chunk, conn)
        });

        let conn = client_endpoint
            .connect(bound_addr, "localhost")
            .unwrap()
            .await
            .unwrap();
        let (mut send_stream, mut recv_stream) = conn.open_bi().await.unwrap();

        let dispatcher = RemoteInputDispatcher::new();
        let query = CapabilityQuery::negotiated(CapabilityId::REMOTE_INPUT, true);

        let chunk = TextInputChunk {
            text: "Hello from phone dictation".into(),
            commit: true,
            sequence: 1,
        };

        let ack = dispatcher
            .send_chunk(&mut send_stream, &mut recv_stream, chunk, &query)
            .await
            .unwrap();

        assert!(ack.success);

        let (mut unneg_send, mut unneg_recv) = conn.open_bi().await.unwrap();
        let unnegotiated_query = CapabilityQuery::negotiated(CapabilityId::REMOTE_INPUT, false);
        let err = dispatcher
            .send_chunk(
                &mut unneg_send,
                &mut unneg_recv,
                TextInputChunk::default(),
                &unnegotiated_query,
            )
            .await;
        assert!(matches!(
            err,
            Err(SessionError::Capability(
                capabilities::CapabilityError::PeerDenied(_)
            ))
        ));

        let (server_res, _server_conn) = server_task.await.unwrap();
        let received = server_res.unwrap();
        assert_eq!(received.text, "Hello from phone dictation");
        assert!(received.commit);
        assert_eq!(received.sequence, 1);
    }

    #[test]
    fn validates_chunk_length() {
        let ok_chunk = TextInputChunk {
            text: "Valid text".into(),
            commit: false,
            sequence: 0,
        };
        assert!(RemoteInputDispatcher::validate_chunk(&ok_chunk).is_ok());

        let oversized_chunk = TextInputChunk {
            text: "a".repeat(MAX_REMOTE_INPUT_TEXT_BYTES + 1),
            commit: true,
            sequence: 1,
        };
        assert!(matches!(
            RemoteInputDispatcher::validate_chunk(&oversized_chunk),
            Err(SessionError::RemoteInputValidation(_))
        ));
    }
}
