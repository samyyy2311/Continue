// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::MAX_FRAME_RING_BYTES;
use protocol::v1::{ring_envelope::Payload, RingAck, RingEnvelope, RingRequest};
use transport::{read_msg, write_msg};

use crate::error::SessionError;

pub const DEFAULT_RING_DURATION_SECS: u32 = 30;

#[derive(Default, Clone)]
pub struct RingDispatcher;

impl RingDispatcher {
    pub fn new() -> Self {
        Self
    }

    /// Sends a ring device request to the remote peer over the QUIC stream and awaits confirmation.
    pub async fn trigger_ring(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        request: RingRequest,
        query: &CapabilityQuery,
    ) -> Result<RingAck, SessionError> {
        evaluate_capability(query)?;

        let envelope = RingEnvelope {
            payload: Some(Payload::Request(request)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_RING_BYTES).await?;
        let ack: RingAck = read_msg(recv_stream, MAX_FRAME_RING_BYTES).await?;

        Ok(ack)
    }

    /// Receives a ring request from the remote peer, delegates to the handler, and writes back confirmation.
    pub async fn receive_envelope<H>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        mut on_ring: H,
    ) -> Result<RingRequest, SessionError>
    where
        H: FnMut(RingRequest) -> RingAck,
    {
        evaluate_capability(query)?;

        let envelope: RingEnvelope = read_msg(recv_stream, MAX_FRAME_RING_BYTES).await?;

        match envelope.payload {
            Some(Payload::Request(request)) => {
                let ack = on_ring(request);
                write_msg(send_stream, &ack, MAX_FRAME_RING_BYTES).await?;
                Ok(request)
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
    async fn ring_trigger_and_receive_roundtrip() {
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

            let dispatcher = RingDispatcher::new();
            let query = CapabilityQuery::negotiated(CapabilityId::RING_DEVICE, true);

            let req = dispatcher
                .receive_envelope(&mut send_stream, &mut recv_stream, &query, |req| RingAck {
                    is_ringing: req.active,
                    status_message: "Alarm playing".into(),
                })
                .await;

            (req, conn)
        });

        let conn = client_endpoint
            .connect(bound_addr, "localhost")
            .unwrap()
            .await
            .unwrap();
        let (mut send_stream, mut recv_stream) = conn.open_bi().await.unwrap();

        let dispatcher = RingDispatcher::new();
        let query = CapabilityQuery::negotiated(CapabilityId::RING_DEVICE, true);

        let req = RingRequest {
            active: true,
            duration_secs: 15,
            force_max_volume: true,
        };

        let ack = dispatcher
            .trigger_ring(&mut send_stream, &mut recv_stream, req, &query)
            .await
            .unwrap();

        assert!(ack.is_ringing);
        assert_eq!(ack.status_message, "Alarm playing");

        let (mut unneg_send, mut unneg_recv) = conn.open_bi().await.unwrap();
        let unnegotiated_query = CapabilityQuery::negotiated(CapabilityId::RING_DEVICE, false);
        let err = dispatcher
            .trigger_ring(
                &mut unneg_send,
                &mut unneg_recv,
                RingRequest::default(),
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
        let req = server_res.unwrap();
        assert!(req.active);
        assert_eq!(req.duration_secs, 15);
        assert!(req.force_max_volume);
    }
}
