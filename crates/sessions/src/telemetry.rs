// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use capabilities::{evaluate_capability, CapabilityQuery};
use limits::MAX_FRAME_TELEMETRY_BYTES;
use protocol::v1::{telemetry_envelope::Payload, DeviceTelemetry, TelemetryAck, TelemetryEnvelope};
use transport::{read_msg, write_msg};

use crate::error::SessionError;

pub const DEFAULT_LOW_BATTERY_THRESHOLD_PERCENT: u32 = 15;

#[derive(Default)]
pub struct TelemetryDispatcher;

impl TelemetryDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub async fn publish_telemetry(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        telemetry: DeviceTelemetry,
        query: &CapabilityQuery,
    ) -> Result<TelemetryAck, SessionError> {
        evaluate_capability(query)?;

        let envelope = TelemetryEnvelope {
            payload: Some(Payload::Telemetry(telemetry)),
        };

        write_msg(send_stream, &envelope, MAX_FRAME_TELEMETRY_BYTES).await?;
        let ack: TelemetryAck = read_msg(recv_stream, MAX_FRAME_TELEMETRY_BYTES).await?;

        Ok(ack)
    }

    pub async fn receive_envelope<H>(
        &self,
        send_stream: &mut quinn::SendStream,
        recv_stream: &mut quinn::RecvStream,
        query: &CapabilityQuery,
        mut on_telemetry: H,
    ) -> Result<DeviceTelemetry, SessionError>
    where
        H: FnMut(DeviceTelemetry) -> Result<(), String>,
    {
        evaluate_capability(query)?;

        let envelope: TelemetryEnvelope = read_msg(recv_stream, MAX_FRAME_TELEMETRY_BYTES).await?;

        match envelope.payload {
            Some(Payload::Telemetry(telemetry)) => {
                let success = on_telemetry(telemetry.clone()).is_ok();
                let ack = TelemetryAck { success };
                write_msg(send_stream, &ack, MAX_FRAME_TELEMETRY_BYTES).await?;
                Ok(telemetry)
            }
            Some(Payload::Ack(_)) | None => Err(SessionError::UnexpectedMessage),
        }
    }
}

pub struct TelemetryAlertEvaluator {
    pub low_battery_threshold: u32,
    alerted_low_battery: bool,
}

impl Default for TelemetryAlertEvaluator {
    fn default() -> Self {
        Self {
            low_battery_threshold: DEFAULT_LOW_BATTERY_THRESHOLD_PERCENT,
            alerted_low_battery: false,
        }
    }
}

impl TelemetryAlertEvaluator {
    pub fn new(threshold: u32) -> Self {
        Self {
            low_battery_threshold: threshold,
            alerted_low_battery: false,
        }
    }

    pub fn evaluate_low_battery(&mut self, level: u32, is_charging: bool) -> bool {
        if is_charging || level > self.low_battery_threshold {
            self.alerted_low_battery = false;
            return false;
        }

        if !self.alerted_low_battery && level <= self.low_battery_threshold {
            self.alerted_low_battery = true;
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::v1::{BatteryState, ChargingType, RadioState};
    use protocol::CapabilityId;
    use std::collections::HashSet;
    use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

    fn test_query(authorized: bool) -> CapabilityQuery {
        let mut caps = HashSet::new();
        caps.insert(CapabilityId::TELEMETRY);
        CapabilityQuery {
            capability: CapabilityId::TELEMETRY,
            is_os_available: true,
            is_app_permitted: true,
            is_peer_authorized: authorized,
            negotiated_session_capabilities: caps,
        }
    }

    #[tokio::test]
    async fn telemetry_publish_receive_roundtrip() {
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

            let dispatcher = TelemetryDispatcher::new();
            let q = test_query(true);

            let res = dispatcher
                .receive_envelope(&mut send, &mut recv, &q, |t| {
                    let bat = t.battery.unwrap();
                    assert_eq!(bat.level_percent, 85);
                    assert!(bat.is_charging);
                    assert_eq!(bat.charging_type, ChargingType::Ac as i32);
                    Ok(())
                })
                .await;

            (res, conn)
        });

        let conn = client
            .connect(server_addr, "continue-device")
            .unwrap()
            .await
            .unwrap();
        let (mut send, mut recv) = conn.open_bi().await.unwrap();

        let dispatcher = TelemetryDispatcher::new();
        let q = test_query(true);

        let telemetry = DeviceTelemetry {
            device_id: "phone-42".into(),
            timestamp_ms: 1000,
            battery: Some(BatteryState {
                level_percent: 85,
                is_charging: true,
                charging_type: ChargingType::Ac as i32,
                is_low_power: false,
            }),
            radio: Some(RadioState {
                wifi_connected: true,
                wifi_ssid: "HomeNet".into(),
                wifi_rssi_dbm: -55,
                cellular_connected: true,
                cellular_signal_level: 4,
                cellular_operator: "Carrier".into(),
            }),
        };

        let ack = dispatcher
            .publish_telemetry(&mut send, &mut recv, telemetry, &q)
            .await
            .expect("telemetry published");

        assert!(ack.success);

        let (server_res, _server_conn) = server_task.await.unwrap();
        server_res.unwrap();
    }

    #[test]
    fn low_battery_evaluator_fires_once_and_resets_on_charge() {
        let mut evaluator = TelemetryAlertEvaluator::new(15);

        // 20% discharging: no alert
        assert!(!evaluator.evaluate_low_battery(20, false));

        // Drops to 15% discharging: fires once
        assert!(evaluator.evaluate_low_battery(15, false));

        // Stays at 14% discharging: does not fire repeatedly
        assert!(!evaluator.evaluate_low_battery(14, false));

        // Plugged into charger: resets state
        assert!(!evaluator.evaluate_low_battery(14, true));

        // Unplugged at 14%: fires again
        assert!(evaluator.evaluate_low_battery(14, false));
    }
}
