// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use mdns_sd::{Receiver, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};

use crate::error::DiscoveryError;

pub const ADB_TLS_PAIRING_SERVICE: &str = "_adb-tls-pairing._tcp.local.";
pub const ADB_TLS_CONNECT_SERVICE: &str = "_adb-tls-connect._tcp.local.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdbServiceType {
    Pairing,
    Connect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredAdbService {
    pub service_type: AdbServiceType,
    pub instance_name: String,
    pub addresses: Vec<SocketAddr>,
    pub port: u16,
}

pub struct AdbDiscoveryBrowser {
    daemon: ServiceDaemon,
    receiver: Receiver<ServiceEvent>,
    service_type: AdbServiceType,
}

impl AdbDiscoveryBrowser {
    pub fn start(service_type: AdbServiceType) -> Result<Self, DiscoveryError> {
        let service_name = match service_type {
            AdbServiceType::Pairing => ADB_TLS_PAIRING_SERVICE,
            AdbServiceType::Connect => ADB_TLS_CONNECT_SERVICE,
        };
        let daemon = ServiceDaemon::new()?;
        let receiver = daemon.browse(service_name)?;
        Ok(Self {
            daemon,
            receiver,
            service_type,
        })
    }

    pub async fn next_service(&self) -> Option<DiscoveredAdbService> {
        loop {
            match self.receiver.recv_async().await {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    return Some(DiscoveredAdbService::from_service_info(
                        &info,
                        self.service_type,
                    ))
                }
                Ok(_) => continue,
                Err(_) => return None,
            }
        }
    }
}

impl Drop for AdbDiscoveryBrowser {
    fn drop(&mut self) {
        let _ = self.daemon.shutdown();
    }
}

impl DiscoveredAdbService {
    pub fn from_service_info(info: &ServiceInfo, service_type: AdbServiceType) -> Self {
        let port = info.get_port();
        let addresses: Vec<SocketAddr> = info
            .get_addresses()
            .iter()
            .map(|ip: &IpAddr| SocketAddr::new(*ip, port))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        Self {
            service_type,
            instance_name: info.get_fullname().to_string(),
            addresses,
            port,
        }
    }
}
