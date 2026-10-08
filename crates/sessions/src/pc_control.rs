// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::MAX_FRAME_PC_CONTROL_BYTES;
use protocol::v1::{
    pc_control_envelope::Payload, PcActionRequest, PcActionResponse, PcControlEnvelope,
};
use transport::{read_msg, write_msg};

use crate::error::SessionError;

#[derive(Default, Clone)]
pub struct PcControlDispatcher;

impl PcControlDispatcher {
    pub fn new() -> Self {
        Self
    }

    /// Sends a remote PC action request to the workstation and awaits the response.
    pub async fn send_action(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        request: PcActionRequest,
        query: &CapabilityQuery,
    ) -> Result<PcActionResponse, SessionError> {
        evaluate_capability(query)?;

        let envelope = PcControlEnvelope {
            payload: Some(Payload::Request(request)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_PC_CONTROL_BYTES).await?;
        let response: PcActionResponse = read_msg(recv_stream, MAX_FRAME_PC_CONTROL_BYTES).await?;

        Ok(response)
    }

    /// Receives a remote PC action request, evaluates capability permissions, delegates to handler, and responds.
    pub async fn receive_envelope<H>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        mut on_action: H,
    ) -> Result<PcActionRequest, SessionError>
    where
        H: FnMut(PcActionRequest) -> PcActionResponse,
    {
        evaluate_capability(query)?;

        let envelope: PcControlEnvelope = read_msg(recv_stream, MAX_FRAME_PC_CONTROL_BYTES).await?;

        match envelope.payload {
            Some(Payload::Request(request)) => {
                let response = on_action(request);
                write_msg(send_stream, &response, MAX_FRAME_PC_CONTROL_BYTES).await?;
                Ok(request)
            }
            Some(Payload::Response(_)) | None => Err(SessionError::UnexpectedMessage),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::v1::PcAction;
    use protocol::CapabilityId;
    use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

    #[tokio::test]
    async fn pc_control_roundtrip() {
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

            let dispatcher = PcControlDispatcher::new();
            let query = CapabilityQuery::negotiated(CapabilityId::PC_CONTROL, true);

            let req = dispatcher
                .receive_envelope(&mut send_stream, &mut recv_stream, &query, |req| {
                    if req.action == (PcAction::LockWorkstation as i32) {
                        PcActionResponse {
                            success: true,
                            error_message: String::new(),
                        }
                    } else {
                        PcActionResponse {
                            success: false,
                            error_message: "Unsupported action".into(),
                        }
                    }
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

        let dispatcher = PcControlDispatcher::new();
        let query = CapabilityQuery::negotiated(CapabilityId::PC_CONTROL, true);

        let req = PcActionRequest {
            action: PcAction::LockWorkstation as i32,
            force: true,
        };

        let resp = dispatcher
            .send_action(&mut send_stream, &mut recv_stream, req, &query)
            .await
            .unwrap();

        assert!(resp.success);
        assert!(resp.error_message.is_empty());

        let (mut unneg_send, mut unneg_recv) = conn.open_bi().await.unwrap();
        let unnegotiated_query = CapabilityQuery::negotiated(CapabilityId::PC_CONTROL, false);
        let err = dispatcher
            .send_action(
                &mut unneg_send,
                &mut unneg_recv,
                PcActionRequest::default(),
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
        assert_eq!(req.action, PcAction::LockWorkstation as i32);
        assert!(req.force);
    }
}
