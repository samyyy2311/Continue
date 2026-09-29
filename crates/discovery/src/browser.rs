// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};

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

    /// Wait for the next peer to be seen on the network. Returns `None` once browsing
    /// has stopped.
    pub async fn next_peer(&self) -> Option<DiscoveredPeer> {
        loop {
            match self.receiver.recv_async().await {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    return Some(DiscoveredPeer::from(&info))
                }
                Ok(_) => continue,
                Err(_) => return None,
            }
        }
    }
}

impl Drop for DiscoveryBrowser {
    fn drop(&mut self) {
        let _ = self.daemon.shutdown();
    }
}

impl From<&ServiceInfo> for DiscoveredPeer {
    fn from(info: &ServiceInfo) -> Self {
        let port = info.get_port();
        let ips: &HashSet<IpAddr> = info.get_addresses();
        let mut addresses: Vec<SocketAddr> =
            ips.iter().map(|&ip| SocketAddr::new(ip, port)).collect();
        // IPv4 first: IPv6 link-local addresses often can't be dialed and only time out.
        addresses.sort_by_key(|addr| addr.is_ipv6());

        let ephemeral_id = info
            .get_property_val_str("id")
            .unwrap_or_else(|| info.get_fullname())
            .to_string();

        let protocol_version = info
            .get_property_val_str("v")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1);

        Self {
            ephemeral_id,
            addresses,
            port,
            protocol_version,
        }
    }
}
