// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent};

use crate::advertiser::SERVICE_TYPE;
use crate::error::DiscoveryError;

/// Discovered Continue peer on the local network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredPeer {
    /// Advertised 128-bit ephemeral discovery ID in hex.
    pub ephemeral_id: String,
    /// Resolved IP addresses of the peer.
    pub addresses: Vec<SocketAddr>,
    /// QUIC port.
    pub port: u16,
    /// Application protocol version.
    pub protocol_version: u32,
}

/// Discovers Continue peers on the local network via mDNS.
pub struct DiscoveryBrowser {
    daemon: ServiceDaemon,
    receiver: Receiver<ServiceEvent>,
}

impl DiscoveryBrowser {
    /// Start browsing for Continue peers on the local network.
    pub fn start() -> Result<Self, DiscoveryError> {
        let daemon = ServiceDaemon::new()?;
        let receiver = daemon.browse(SERVICE_TYPE)?;
        Ok(Self { daemon, receiver })
    }

    /// Receive the next discovery event, extracting peer details if a service was resolved.
    pub fn recv(&self) -> Result<Option<DiscoveredPeer>, DiscoveryError> {
        match self.receiver.recv() {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let port = info.get_port();
                let ips: &HashSet<IpAddr> = info.get_addresses();
                let addresses: Vec<SocketAddr> = ips.iter().map(|&ip| SocketAddr::new(ip, port)).collect();

                let ephemeral_id = info
                    .get_property_val_str("id")
                    .unwrap_or_else(|| info.get_name())
                    .to_string();

                let protocol_version = info
                    .get_property_val_str("v")
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(1);

                Ok(Some(DiscoveredPeer {
                    ephemeral_id,
                    addresses,
                    port,
                    protocol_version,
                }))
            }
            Ok(ServiceEvent::ServiceRemoved(_, _)) => Ok(None),
            Ok(_) => Ok(None),
            Err(_) => Err(DiscoveryError::Shutdown),
        }
    }

    /// Stop browsing for peers.
    pub fn stop(self) -> Result<(), DiscoveryError> {
        self.daemon.stop_browse(SERVICE_TYPE)?;
        Ok(())
    }
}
