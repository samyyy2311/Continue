// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::{MAX_DECK_LABEL_BYTES, MAX_DECK_TILES, MAX_DECK_TILE_ID_BYTES, MAX_FRAME_DECK_BYTES};
use protocol::v1::{
    deck_envelope::Payload, DeckAck, DeckEnvelope, DeckLayoutSync, DeckTileConfig, DeckTriggerEvent,
};
use transport::{read_msg, write_msg};

use crate::error::SessionError;

#[derive(Default, Clone)]
pub struct DeckDispatcher;

impl DeckDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_layout(layout: &DeckLayoutSync) -> Result<(), SessionError> {
        if layout.tiles.len() > MAX_DECK_TILES {
            return Err(SessionError::DeckValidation(format!(
                "Tile count {} exceeds maximum allowed of {}",
                layout.tiles.len(),
                MAX_DECK_TILES
            )));
        }

        for tile in &layout.tiles {
            Self::validate_tile(tile)?;
        }

        Ok(())
    }

    pub fn validate_tile(tile: &DeckTileConfig) -> Result<(), SessionError> {
        if tile.tile_id.is_empty() {
            return Err(SessionError::DeckValidation(
                "Deck tile ID cannot be empty".into(),
            ));
        }

        if tile.tile_id.len() > MAX_DECK_TILE_ID_BYTES {
            return Err(SessionError::DeckValidation(format!(
                "Tile ID length {} exceeds maximum allowed of {} bytes",
                tile.tile_id.len(),
                MAX_DECK_TILE_ID_BYTES
            )));
        }

        if tile.label.len() > MAX_DECK_LABEL_BYTES {
            return Err(SessionError::DeckValidation(format!(
                "Tile label length {} exceeds maximum allowed of {} bytes",
                tile.label.len(),
                MAX_DECK_LABEL_BYTES
            )));
        }

        Ok(())
    }

    pub fn validate_trigger(trigger: &DeckTriggerEvent) -> Result<(), SessionError> {
        if trigger.tile_id.is_empty() {
            return Err(SessionError::DeckValidation(
                "Deck trigger tile ID cannot be empty".into(),
            ));
        }

        if trigger.tile_id.len() > MAX_DECK_TILE_ID_BYTES {
            return Err(SessionError::DeckValidation(format!(
                "Trigger tile ID length {} exceeds maximum allowed of {} bytes",
                trigger.tile_id.len(),
                MAX_DECK_TILE_ID_BYTES
            )));
        }

        Ok(())
    }

    /// Pushes a deck layout from the host to the connected peer device.
    pub async fn sync_layout(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        layout: DeckLayoutSync,
        query: &CapabilityQuery,
    ) -> Result<DeckAck, SessionError> {
        evaluate_capability(query)?;
        Self::validate_layout(&layout)?;

        let envelope = DeckEnvelope {
            payload: Some(Payload::Layout(layout)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_DECK_BYTES).await?;
        let ack: DeckAck = read_msg(recv_stream, MAX_FRAME_DECK_BYTES).await?;

        Ok(ack)
    }

    /// Sends a tile button press/trigger event from the peer to the host workstation.
    pub async fn trigger_tile(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        trigger: DeckTriggerEvent,
        query: &CapabilityQuery,
    ) -> Result<DeckAck, SessionError> {
        evaluate_capability(query)?;
        Self::validate_trigger(&trigger)?;

        let envelope = DeckEnvelope {
            payload: Some(Payload::Trigger(trigger)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_DECK_BYTES).await?;
        let ack: DeckAck = read_msg(recv_stream, MAX_FRAME_DECK_BYTES).await?;

        Ok(ack)
    }

    /// Receives a deck envelope, delegates layout or trigger handling, and replies with an ack.
    pub async fn receive_envelope<L, T>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        mut on_layout: L,
        mut on_trigger: T,
    ) -> Result<DeckEnvelope, SessionError>
    where
        L: FnMut(DeckLayoutSync) -> DeckAck,
        T: FnMut(DeckTriggerEvent) -> DeckAck,
    {
        evaluate_capability(query)?;

        let envelope: DeckEnvelope = read_msg(recv_stream, MAX_FRAME_DECK_BYTES).await?;

        match envelope.payload {
            Some(Payload::Layout(ref layout)) => {
                Self::validate_layout(layout)?;
                let ack = on_layout(layout.clone());
                write_msg(send_stream, &ack, MAX_FRAME_DECK_BYTES).await?;
                Ok(envelope)
            }
            Some(Payload::Trigger(ref trigger)) => {
                Self::validate_trigger(trigger)?;
                let ack = on_trigger(trigger.clone());
                write_msg(send_stream, &ack, MAX_FRAME_DECK_BYTES).await?;
                Ok(envelope)
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
    async fn deck_sync_layout_roundtrip() {
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

            let dispatcher = DeckDispatcher::new();
            let query = CapabilityQuery::negotiated(CapabilityId::DECK, true);

            let res = dispatcher
                .receive_envelope(
                    &mut send_stream,
                    &mut recv_stream,
                    &query,
                    |layout| DeckAck {
                        success: layout.tiles.len() == 2,
                        error_message: String::new(),
                    },
                    |_| unreachable!(),
                )
                .await;

            (res, conn)
        });

        let conn = client_endpoint
            .connect(bound_addr, "localhost")
            .unwrap()
            .await
            .unwrap();
        let (mut send_stream, mut recv_stream) = conn.open_bi().await.unwrap();

        let dispatcher = DeckDispatcher::new();
        let query = CapabilityQuery::negotiated(CapabilityId::DECK, true);

        let layout = DeckLayoutSync {
            tiles: vec![
                DeckTileConfig {
                    tile_id: "mute_mic".into(),
                    label: "Mute Mic".into(),
                    icon: "mic-off".into(),
                    action_command: "toggle_mic".into(),
                },
                DeckTileConfig {
                    tile_id: "screen_lock".into(),
                    label: "Lock PC".into(),
                    icon: "lock".into(),
                    action_command: "lock".into(),
                },
            ],
        };

        let ack = dispatcher
            .sync_layout(&mut send_stream, &mut recv_stream, layout, &query)
            .await
            .unwrap();

        assert!(ack.success);

        let (mut unneg_send, mut unneg_recv) = conn.open_bi().await.unwrap();
        let unnegotiated_query = CapabilityQuery::negotiated(CapabilityId::DECK, false);
        let err = dispatcher
            .sync_layout(
                &mut unneg_send,
                &mut unneg_recv,
                DeckLayoutSync::default(),
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
        let env = server_res.unwrap();
        assert!(matches!(env.payload, Some(Payload::Layout(_))));
    }

    #[tokio::test]
    async fn deck_trigger_roundtrip() {
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

            let dispatcher = DeckDispatcher::new();
            let query = CapabilityQuery::negotiated(CapabilityId::DECK, true);

            let res = dispatcher
                .receive_envelope(
                    &mut send_stream,
                    &mut recv_stream,
                    &query,
                    |_| unreachable!(),
                    |trigger| DeckAck {
                        success: trigger.tile_id == "play_pause",
                        error_message: String::new(),
                    },
                )
                .await;

            (res, conn)
        });

        let conn = client_endpoint
            .connect(bound_addr, "localhost")
            .unwrap()
            .await
            .unwrap();
        let (mut send_stream, mut recv_stream) = conn.open_bi().await.unwrap();

        let dispatcher = DeckDispatcher::new();
        let query = CapabilityQuery::negotiated(CapabilityId::DECK, true);

        let trigger = DeckTriggerEvent {
            tile_id: "play_pause".into(),
        };

        let ack = dispatcher
            .trigger_tile(&mut send_stream, &mut recv_stream, trigger, &query)
            .await
            .unwrap();

        assert!(ack.success);

        let (server_res, _server_conn) = server_task.await.unwrap();
        let env = server_res.unwrap();
        assert!(matches!(env.payload, Some(Payload::Trigger(_))));
    }

    #[test]
    fn validates_tile_and_trigger_constraints() {
        let empty_trigger = DeckTriggerEvent {
            tile_id: String::new(),
        };
        assert!(matches!(
            DeckDispatcher::validate_trigger(&empty_trigger),
            Err(SessionError::DeckValidation(_))
        ));

        let valid_tile = DeckTileConfig {
            tile_id: "tile_1".into(),
            label: "Action".into(),
            icon: "star".into(),
            action_command: "cmd".into(),
        };
        assert!(DeckDispatcher::validate_tile(&valid_tile).is_ok());

        let oversized_label_tile = DeckTileConfig {
            tile_id: "tile_1".into(),
            label: "x".repeat(MAX_DECK_LABEL_BYTES + 1),
            icon: "star".into(),
            action_command: "cmd".into(),
        };
        assert!(matches!(
            DeckDispatcher::validate_tile(&oversized_label_tile),
            Err(SessionError::DeckValidation(_))
        ));
    }
}
