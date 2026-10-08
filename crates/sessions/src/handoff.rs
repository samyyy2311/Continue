// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::{MAX_FRAME_HANDOFF_BYTES, MAX_HANDOFF_TITLE_BYTES, MAX_HANDOFF_URI_BYTES};
use protocol::v1::{
    handoff_envelope::Payload, HandoffAck, HandoffBroadcast, HandoffDismiss, HandoffEnvelope,
    HandoffItem,
};
use transport::{read_msg, write_msg};

use crate::error::SessionError;

#[derive(Default)]
pub struct HandoffDispatcher;

impl HandoffDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_item(item: &HandoffItem) -> Result<(), SessionError> {
        if item.uri.len() > MAX_HANDOFF_URI_BYTES {
            return Err(SessionError::HandoffValidation(format!(
                "Handoff URI exceeds maximum allowed length of {} bytes",
                MAX_HANDOFF_URI_BYTES
            )));
        }

        if item.title.len() > MAX_HANDOFF_TITLE_BYTES {
            return Err(SessionError::HandoffValidation(format!(
                "Handoff title exceeds maximum allowed length of {} bytes",
                MAX_HANDOFF_TITLE_BYTES
            )));
        }

        Ok(())
    }

    pub async fn broadcast_handoff(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        item: HandoffItem,
        query: &CapabilityQuery,
    ) -> Result<HandoffAck, SessionError> {
        evaluate_capability(query)?;
        Self::validate_item(&item)?;

        let envelope = HandoffEnvelope {
            payload: Some(Payload::Broadcast(HandoffBroadcast { item: Some(item) })),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_HANDOFF_BYTES).await?;
        let ack: HandoffAck = read_msg(recv_stream, MAX_FRAME_HANDOFF_BYTES).await?;

        if !ack.success {
            return Err(SessionError::HandoffRejected(ack.error_message));
        }

        Ok(ack)
    }

    pub async fn dismiss_handoff(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        handoff_id: String,
        query: &CapabilityQuery,
    ) -> Result<HandoffAck, SessionError> {
        evaluate_capability(query)?;

        let envelope = HandoffEnvelope {
            payload: Some(Payload::Dismiss(HandoffDismiss {
                handoff_id: handoff_id.clone(),
            })),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_HANDOFF_BYTES).await?;
        let ack: HandoffAck = read_msg(recv_stream, MAX_FRAME_HANDOFF_BYTES).await?;

        if !ack.success {
            return Err(SessionError::HandoffRejected(ack.error_message));
        }

        Ok(ack)
    }

    pub async fn receive_envelope<HB, HD>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        mut on_broadcast: HB,
        mut on_dismiss: HD,
    ) -> Result<HandoffEnvelope, SessionError>
    where
        HB: FnMut(HandoffItem) -> Result<(), String>,
        HD: FnMut(String) -> Result<(), String>,
    {
        evaluate_capability(query)?;

        let envelope: HandoffEnvelope = read_msg(recv_stream, MAX_FRAME_HANDOFF_BYTES).await?;

        match envelope.payload.clone() {
            Some(Payload::Broadcast(broadcast)) => {
                let item = broadcast.item.ok_or(SessionError::UnexpectedMessage)?;
                let handoff_id = item.handoff_id.clone();
                let ack = match on_broadcast(item) {
                    Ok(()) => HandoffAck {
                        handoff_id,
                        success: true,
                        error_message: String::new(),
                    },
                    Err(err) => HandoffAck {
                        handoff_id,
                        success: false,
                        error_message: err,
                    },
                };
                write_msg(send_stream, &ack, MAX_FRAME_HANDOFF_BYTES).await?;
            }
            Some(Payload::Dismiss(dismiss)) => {
                let handoff_id = dismiss.handoff_id;
                let ack = match on_dismiss(handoff_id.clone()) {
                    Ok(()) => HandoffAck {
                        handoff_id,
                        success: true,
                        error_message: String::new(),
                    },
                    Err(err) => HandoffAck {
                        handoff_id,
                        success: false,
                        error_message: err,
                    },
                };
                write_msg(send_stream, &ack, MAX_FRAME_HANDOFF_BYTES).await?;
            }
            Some(Payload::Ack(_)) | None => {
                return Err(SessionError::UnexpectedMessage);
            }
        }

        Ok(envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::v1::HandoffType;
    use protocol::CapabilityId;
    use std::collections::HashSet;
    use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

    fn test_query(authorized: bool) -> CapabilityQuery {
        let mut caps = HashSet::new();
        caps.insert(CapabilityId::HANDOFF);
        CapabilityQuery {
            capability: CapabilityId::HANDOFF,
            is_os_available: true,
            is_app_permitted: true,
            is_peer_authorized: authorized,
            negotiated_session_capabilities: caps,
        }
    }

    #[tokio::test]
    async fn handoff_broadcast_roundtrip() {
        let server_cert = TransportCertificate::generate().unwrap();
        let client_cert = TransportCertificate::generate().unwrap();

        let server_tls = server_cert
            .build_pinned_server_tls(client_cert.spki_hash)
            .unwrap();
        let client_tls = client_cert
            .build_pinned_client_tls(server_cert.spki_hash)
            .unwrap();

        let server = create_server_endpoint("127.0.0.1:0".parse().unwrap(), server_tls).unwrap();
        let server_addr = server.local_addr().unwrap();
        let client = create_client_endpoint("127.0.0.1:0".parse().unwrap(), client_tls).unwrap();

        let server_task = tokio::spawn(async move {
            let incoming = server.accept().await.unwrap();
            let conn = incoming.await.unwrap();
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();

            let dispatcher = HandoffDispatcher::new();
            let q = test_query(true);

            let res = dispatcher
                .receive_envelope(
                    &mut send,
                    &mut recv,
                    &q,
                    |item| {
                        assert_eq!(item.handoff_id, "h-1");
                        assert_eq!(item.uri, "https://github.com/continue/project");
                        assert_eq!(item.handoff_type, HandoffType::Url as i32);
                        Ok(())
                    },
                    |_| Ok(()),
                )
                .await;

            (res, conn)
        });

        let conn = client
            .connect(server_addr, "continue-device")
            .unwrap()
            .await
            .unwrap();
        let (mut send, mut recv) = conn.open_bi().await.unwrap();

        let dispatcher = HandoffDispatcher::new();
        let q = test_query(true);

        let item = HandoffItem {
            handoff_id: "h-1".into(),
            source_device_id: "phone-1".into(),
            handoff_type: HandoffType::Url as i32,
            title: "Continue Project".into(),
            uri: "https://github.com/continue/project".into(),
            scroll_ratio: 0.42,
            cursor_position: 128,
            timestamp_ms: 1700000000,
            extra_payload: Vec::new(),
        };

        let ack = dispatcher
            .broadcast_handoff(&mut send, &mut recv, item, &q)
            .await
            .expect("broadcast delivered");

        assert_eq!(ack.handoff_id, "h-1");
        assert!(ack.success);

        let (server_res, _server_conn) = server_task.await.unwrap();
        server_res.unwrap();
    }

    #[tokio::test]
    async fn handoff_dismiss_roundtrip() {
        let server_cert = TransportCertificate::generate().unwrap();
        let client_cert = TransportCertificate::generate().unwrap();

        let server_tls = server_cert
            .build_pinned_server_tls(client_cert.spki_hash)
            .unwrap();
        let client_tls = client_cert
            .build_pinned_client_tls(server_cert.spki_hash)
            .unwrap();

        let server = create_server_endpoint("127.0.0.1:0".parse().unwrap(), server_tls).unwrap();
        let server_addr = server.local_addr().unwrap();
        let client = create_client_endpoint("127.0.0.1:0".parse().unwrap(), client_tls).unwrap();

        let server_task = tokio::spawn(async move {
            let incoming = server.accept().await.unwrap();
            let conn = incoming.await.unwrap();
            let (mut send, mut recv) = conn.accept_bi().await.unwrap();

            let dispatcher = HandoffDispatcher::new();
            let q = test_query(true);

            let res = dispatcher
                .receive_envelope(
                    &mut send,
                    &mut recv,
                    &q,
                    |_| Ok(()),
                    |handoff_id| {
                        assert_eq!(handoff_id, "h-42");
                        Ok(())
                    },
                )
                .await;

            (res, conn)
        });

        let conn = client
            .connect(server_addr, "continue-device")
            .unwrap()
            .await
            .unwrap();
        let (mut send, mut recv) = conn.open_bi().await.unwrap();

        let dispatcher = HandoffDispatcher::new();
        let q = test_query(true);

        let ack = dispatcher
            .dismiss_handoff(&mut send, &mut recv, "h-42".into(), &q)
            .await
            .expect("dismiss delivered");

        assert_eq!(ack.handoff_id, "h-42");
        assert!(ack.success);

        let (server_res, _server_conn) = server_task.await.unwrap();
        server_res.unwrap();
    }

    #[test]
    fn handoff_validation_rejects_oversized_uri() {
        let oversized_uri = "https://example.com/".repeat(300); // > 4096 bytes
        let item = HandoffItem {
            handoff_id: "h-overflow".into(),
            source_device_id: "src".into(),
            handoff_type: HandoffType::Url as i32,
            title: "Test".into(),
            uri: oversized_uri,
            scroll_ratio: 0.0,
            cursor_position: 0,
            timestamp_ms: 0,
            extra_payload: Vec::new(),
        };

        let result = HandoffDispatcher::validate_item(&item);
        assert!(result.is_err());
    }
}
