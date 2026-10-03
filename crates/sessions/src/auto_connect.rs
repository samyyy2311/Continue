// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use discovery::{DiscoveryBrowser, RESTART_DELAY};
use pairing::{TrustStore, TrustedPeer};
use tracing::warn;

use crate::registry::SessionRegistry;

const REDIAL_INTERVAL: Duration = Duration::from_secs(10);

/// Peer and address pairs with a dial in progress. Each loop keeps its own, so a discovered
/// address that turns out to be another device doesn't hold up dialing the saved one, and a
/// slow address doesn't hold up a new one for the same peer.
type Dialing = Arc<Mutex<HashSet<(String, SocketAddr)>>>;

/// Keeps paired devices connected until the future is dropped. Dials them when they show up
/// over mDNS, and every few seconds at the address they were last reached at, which covers
/// networks where discovery doesn't get through, such as a phone's hotspot.
pub async fn connect_paired_peers(registry: SessionRegistry, trust_store: TrustStore) {
    tokio::join!(
        dial_discovered_peers(&registry, &trust_store),
        dial_known_addresses(&registry, &trust_store),
    );
}

async fn dial_discovered_peers(registry: &SessionRegistry, trust_store: &TrustStore) {
    let dialing = Dialing::default();
    loop {
        let browser = match DiscoveryBrowser::start() {
            Ok(browser) => browser,
            Err(error) => {
                warn!("Looking for devices on the local network failed: {error}");
                tokio::time::sleep(RESTART_DELAY).await;
                continue;
            }
        };

        while let Some(found) = browser.next_peer().await {
            for peer in paired_peers(trust_store) {
                dial(
                    registry,
                    trust_store,
                    &dialing,
                    peer,
                    found.addresses.clone(),
                );
            }
        }

        tokio::time::sleep(RESTART_DELAY).await;
    }
}

async fn dial_known_addresses(registry: &SessionRegistry, trust_store: &TrustStore) {
    let dialing = Dialing::default();
    loop {
        for peer in paired_peers(trust_store) {
            let known = trust_store
                .last_endpoint(&peer.fingerprint)
                .ok()
                .flatten()
                .and_then(|endpoint| endpoint.parse::<SocketAddr>().ok());
            if let Some(addr) = known {
                dial(registry, trust_store, &dialing, peer, vec![addr]);
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(REDIAL_INTERVAL) => {}
            _ = registry.redial_requested() => {}
        }
    }
}

fn paired_peers(trust_store: &TrustStore) -> Vec<TrustedPeer> {
    trust_store.list_peers().unwrap_or_else(|error| {
        warn!("Could not read paired devices: {error}");
        Vec::new()
    })
}

fn dial(
    registry: &SessionRegistry,
    trust_store: &TrustStore,
    dialing: &Dialing,
    peer: TrustedPeer,
    addresses: Vec<SocketAddr>,
) {
    let addresses: Vec<SocketAddr> = {
        let mut in_progress = dialing.lock().unwrap();
        addresses
            .into_iter()
            .filter(|addr| in_progress.insert((peer.fingerprint.clone(), *addr)))
            .collect()
    };
    if addresses.is_empty() {
        return;
    }
    let registry = registry.clone();
    let trust_store = trust_store.clone();
    let dialing = dialing.clone();
    tokio::spawn(async move {
        let still_trusted = || {
            matches!(
                trust_store.get_peer(&peer.fingerprint),
                Ok(Some(current)) if current.transport_spki_hash == peer.transport_spki_hash
            )
        };
        let reached = registry
            .connect_discovered(
                &peer.fingerprint,
                peer.transport_spki_hash,
                &addresses,
                still_trusted,
            )
            .await;
        if let Some(addr) = reached {
            if let Err(error) = trust_store.set_last_endpoint(&peer.fingerprint, &addr.to_string())
            {
                warn!(
                    "Could not save the address of {}: {error}",
                    peer.fingerprint
                );
            }
        }
        let mut in_progress = dialing.lock().unwrap();
        for addr in addresses {
            in_progress.remove(&(peer.fingerprint.clone(), addr));
        }
    });
}
