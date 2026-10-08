// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::MAX_FRAME_MEDIA_CONTROL_BYTES;
use protocol::v1::{
    media_control_envelope::Payload, MediaCommandRequest, MediaCommandResponse,
    MediaControlEnvelope, MediaStatusUpdate,
};
use transport::{read_msg, write_msg};

use crate::error::SessionError;

#[derive(Default)]
pub struct MediaControlDispatcher;

impl MediaControlDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub async fn send_command(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        request: MediaCommandRequest,
        query: &CapabilityQuery,
    ) -> Result<MediaCommandResponse, SessionError> {
        evaluate_capability(query)?;

        let envelope = MediaControlEnvelope {
            payload: Some(Payload::CommandRequest(request)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_MEDIA_CONTROL_BYTES).await?;
        let response: MediaCommandResponse =
            read_msg(recv_stream, MAX_FRAME_MEDIA_CONTROL_BYTES).await?;

        if !response.success {
            return Err(SessionError::MediaControlRejected(response.error_message));
        }

        Ok(response)
    }

    pub async fn publish_status(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        update: MediaStatusUpdate,
        query: &CapabilityQuery,
    ) -> Result<MediaCommandResponse, SessionError> {
        evaluate_capability(query)?;

        let envelope = MediaControlEnvelope {
            payload: Some(Payload::StatusUpdate(update)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_MEDIA_CONTROL_BYTES).await?;
        let ack: MediaCommandResponse =
            read_msg(recv_stream, MAX_FRAME_MEDIA_CONTROL_BYTES).await?;

        Ok(ack)
    }

    pub async fn receive_envelope<HC, HS>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        mut on_command: HC,
        mut on_status: HS,
    ) -> Result<MediaControlEnvelope, SessionError>
    where
        HC: FnMut(MediaCommandRequest) -> MediaCommandResponse,
        HS: FnMut(MediaStatusUpdate),
    {
        evaluate_capability(query)?;

        let envelope: MediaControlEnvelope =
            read_msg(recv_stream, MAX_FRAME_MEDIA_CONTROL_BYTES).await?;

        match envelope.payload.clone() {
            Some(Payload::CommandRequest(req)) => {
                let resp = on_command(req);
                write_msg(send_stream, &resp, MAX_FRAME_MEDIA_CONTROL_BYTES).await?;
            }
            Some(Payload::StatusUpdate(stat)) => {
                on_status(stat);
                let ack = MediaCommandResponse {
                    command_id: String::new(),
                    success: true,
                    error_message: String::new(),
                };
                write_msg(send_stream, &ack, MAX_FRAME_MEDIA_CONTROL_BYTES).await?;
            }
            None => {
                return Err(SessionError::UnexpectedMessage);
            }
        }

        Ok(envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::v1::{MediaMetadata, MediaPlaybackCommand, MediaPlaybackState};
    use protocol::CapabilityId;
    use std::collections::HashSet;
    use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

    fn test_query(authorized: bool) -> CapabilityQuery {
        let mut caps = HashSet::new();
        caps.insert(CapabilityId::MEDIA_CONTROL);
        CapabilityQuery {
            capability: CapabilityId::MEDIA_CONTROL,
            is_os_available: true,
            is_app_permitted: true,
            is_peer_authorized: authorized,
            negotiated_session_capabilities: caps,
        }
    }

    #[tokio::test]
    async fn media_control_command_roundtrip() {
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

            let dispatcher = MediaControlDispatcher::new();
            let q = test_query(true);

            let result = dispatcher
                .receive_envelope(
                    &mut send,
                    &mut recv,
                    &q,
                    |cmd| {
                        assert_eq!(cmd.command, MediaPlaybackCommand::Play as i32);
                        MediaCommandResponse {
                            command_id: cmd.command_id,
                            success: true,
                            error_message: String::new(),
                        }
                    },
                    |_| {},
                )
                .await;

            (result, conn)
        });

        let conn = client
            .connect(server_addr, "continue-device")
            .unwrap()
            .await
            .unwrap();
        let (mut send, mut recv) = conn.open_bi().await.unwrap();

        let dispatcher = MediaControlDispatcher::new();
        let q = test_query(true);

        let req = MediaCommandRequest {
            command_id: "cmd-1".into(),
            command: MediaPlaybackCommand::Play as i32,
            seek_position_ms: 0,
            volume: 1.0,
        };

        let resp = dispatcher
            .send_command(&mut send, &mut recv, req, &q)
            .await
            .expect("command delivered");

        assert_eq!(resp.command_id, "cmd-1");
        assert!(resp.success);

        let (server_res, _server_conn) = server_task.await.unwrap();
        server_res.unwrap();
    }

    #[tokio::test]
    async fn media_control_status_update_roundtrip() {
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

            let dispatcher = MediaControlDispatcher::new();
            let q = test_query(true);

            let result = dispatcher
                .receive_envelope(
                    &mut send,
                    &mut recv,
                    &q,
                    |_| unreachable!(),
                    |stat| {
                        let meta = stat.metadata.unwrap();
                        assert_eq!(meta.title, "Track One");
                        assert_eq!(meta.artist, "Artist Name");
                    },
                )
                .await;

            (result, conn)
        });

        let conn = client
            .connect(server_addr, "continue-device")
            .unwrap()
            .await
            .unwrap();
        let (mut send, mut recv) = conn.open_bi().await.unwrap();

        let dispatcher = MediaControlDispatcher::new();
        let q = test_query(true);

        let update = MediaStatusUpdate {
            metadata: Some(MediaMetadata {
                title: "Track One".into(),
                artist: "Artist Name".into(),
                album: "Album Name".into(),
                duration_ms: 210000,
                position_ms: 45000,
                state: MediaPlaybackState::Playing as i32,
                volume: 0.8,
                artwork_thumbnail: Vec::new(),
            }),
            timestamp_ms: 1000,
        };

        let ack = dispatcher
            .publish_status(&mut send, &mut recv, update, &q)
            .await
            .expect("status published");

        assert!(ack.success);

        let (server_res, _server_conn) = server_task.await.unwrap();
        server_res.unwrap();
    }
}
