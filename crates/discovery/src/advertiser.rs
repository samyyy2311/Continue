// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;
use std::time::Duration;
use tracing::warn;

use crate::ephemeral::{EphemeralDiscoveryId, ROTATION_PERIOD};
use crate::error::DiscoveryError;

pub const SERVICE_TYPE: &str = "_continue._udp.local.";

/// How long to wait before trying again when the network isn't available.
pub const RESTART_DELAY: Duration = Duration::from_millis(limits::RETRY_MAX_DELAY_MS);

/// Advertises local device presence over mDNS with an ephemeral discovery ID.
pub struct DiscoveryAdvertiser {
    daemon: ServiceDaemon,
    fullname: String,
}

impl DiscoveryAdvertiser {
    /// Start advertising the Continue QUIC service on the given port.
    pub fn start(
        port: u16,
        ephemeral_id: EphemeralDiscoveryId,
        protocol_version: u32,
    ) -> Result<Self, DiscoveryError> {
        let daemon = ServiceDaemon::new()?;
        let instance_name = ephemeral_id.to_hex();
        let host_name = format!("{}.local.", instance_name);

        let mut properties = HashMap::new();
        properties.insert("v".to_string(), protocol_version.to_string());
        properties.insert("id".to_string(), instance_name.clone());

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &instance_name,
            &host_name,
            "",
            port,
            properties,
        )?
        // Without addresses the service is announced but browsers can never resolve it.
        .enable_addr_auto();

        let fullname = service_info.get_fullname().to_string();
        daemon.register(service_info)?;

        Ok(Self { daemon, fullname })
    }

    /// Stop advertising and unregister the service from the local network.
    pub fn unregister(self) -> Result<(), DiscoveryError> {
        let receiver = self.daemon.unregister(&self.fullname)?;
        let _ = receiver.recv();
        self.daemon.shutdown()?;
        Ok(())
    }
}

/// Keeps this device advertised on `port` for as long as the future runs. Retries when
/// advertising fails, e.g. with no network at startup, and switches to a fresh ID once
/// a day so the device can't be tracked across networks.
pub async fn advertise(port: u16, protocol_version: u32) {
    loop {
        match DiscoveryAdvertiser::start(port, EphemeralDiscoveryId::generate(), protocol_version) {
            Ok(advertiser) => {
                tokio::time::sleep(ROTATION_PERIOD).await;
                // Unregistering waits for the daemon to confirm.
                let _ = tokio::task::spawn_blocking(move || advertiser.unregister()).await;
            }
            Err(error) => {
                warn!("Advertising on the local network failed: {error}");
                tokio::time::sleep(RESTART_DELAY).await;
            }
        }
    }
}
