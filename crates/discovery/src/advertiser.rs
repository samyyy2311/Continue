// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use mdns_sd::{ServiceDaemon, ServiceInfo};

use crate::ephemeral::EphemeralDiscoveryId;
use crate::error::DiscoveryError;

pub const SERVICE_TYPE: &str = "_continue._udp.local.";

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
        )?;

        let fullname = service_info.get_fullname().to_string();
        daemon.register(service_info)?;

        Ok(Self { daemon, fullname })
    }

    /// Stop advertising and unregister the service from the local network.
    pub fn unregister(self) -> Result<(), DiscoveryError> {
        let receiver = self.daemon.unregister(&self.fullname)?;
        let _ = receiver.recv();
        Ok(())
    }
}
