// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! A computer waiting to pair, announced so a phone nearby can pick it from a list instead of
//! scanning its code. Anyone on the network can read the code, so pairing this way is only
//! trusted once people compare the six digits both devices then show.

use std::collections::HashMap;
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::ephemeral::EphemeralDiscoveryId;
use crate::error::DiscoveryError;

pub const PAIRING_SERVICE_TYPE: &str = "_continue-pair._udp.local.";

/// Text record values hold at most 255 bytes, so the code is split across several.
const CODE_PART_BYTES: usize = 200;

/// A computer waiting to pair, as a phone sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NearbyComputer {
    pub name: String,
    pub code: String,
}

/// Stops announcing when dropped.
pub struct PairingAdvertiser {
    daemon: ServiceDaemon,
    fullname: String,
}

impl PairingAdvertiser {
    pub fn start(name: &str, code: &str, port: u16) -> Result<Self, DiscoveryError> {
        let daemon = ServiceDaemon::new()?;
        let instance = EphemeralDiscoveryId::generate().to_hex();
        let mut properties = HashMap::from([("n".to_string(), name.to_string())]);
        for (index, part) in code.as_bytes().chunks(CODE_PART_BYTES).enumerate() {
            properties.insert(
                format!("c{index}"),
                String::from_utf8_lossy(part).into_owned(),
            );
        }
        let host = format!("{instance}.local.");
        let service =
            ServiceInfo::new(PAIRING_SERVICE_TYPE, &instance, &host, "", port, properties)?
                .enable_addr_auto();
        let fullname = service.get_fullname().to_string();
        daemon.register(service)?;
        Ok(Self { daemon, fullname })
    }
}

impl Drop for PairingAdvertiser {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

/// The computers waiting to pair that answer within `wait`.
pub async fn find_nearby(wait: Duration) -> Result<Vec<NearbyComputer>, DiscoveryError> {
    let daemon = ServiceDaemon::new()?;
    let events = daemon.browse(PAIRING_SERVICE_TYPE)?;
    let mut found = Vec::new();
    let _ = tokio::time::timeout(wait, async {
        while let Ok(event) = events.recv_async().await {
            if let ServiceEvent::ServiceResolved(info) = event {
                if let Some(computer) = nearby_computer(&info) {
                    if !found.contains(&computer) {
                        found.push(computer);
                    }
                }
            }
        }
    })
    .await;
    let _ = daemon.shutdown();
    Ok(found)
}

fn nearby_computer(info: &ServiceInfo) -> Option<NearbyComputer> {
    let name = info.get_property_val_str("n")?.to_string();
    let code: String = (0..)
        .map_while(|index| info.get_property_val_str(&format!("c{index}")))
        .collect();
    (!code.is_empty()).then_some(NearbyComputer { name, code })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_code_is_split_across_records_and_read_back_whole() {
        let code = "x".repeat(CODE_PART_BYTES * 2 + 17);
        let mut properties = HashMap::from([("n".to_string(), "Work laptop".to_string())]);
        for (index, part) in code.as_bytes().chunks(CODE_PART_BYTES).enumerate() {
            properties.insert(
                format!("c{index}"),
                String::from_utf8_lossy(part).into_owned(),
            );
        }
        let info =
            ServiceInfo::new(PAIRING_SERVICE_TYPE, "a1", "a1.local.", "", 1, properties).unwrap();

        let found = nearby_computer(&info).unwrap();
        assert_eq!(found.name, "Work laptop");
        assert_eq!(found.code, code);
    }
}
