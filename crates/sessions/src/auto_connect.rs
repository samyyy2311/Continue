// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use discovery::{DiscoveryBrowser, RESTART_DELAY};
use pairing::TrustStore;
use tracing::warn;

use crate::registry::SessionRegistry;

/// Connects to trusted peers as they appear on the local network and saves the address
/// that worked. Runs for as long as the future is polled, restarting browsing if it fails.
pub async fn connect_discovered_peers(registry: SessionRegistry, trust_store: TrustStore) {
    let dialing: Arc<Mutex<HashSet<String>>> = Arc::default();
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
            let peers = match trust_store.list_peers() {
                Ok(peers) => peers,
                Err(error) => {
                    warn!("Could not read paired devices: {error}");
                    continue;
                }
            };
            for peer in peers {
                if !dialing.lock().unwrap().insert(peer.fingerprint.clone()) {
                    continue;
                }
                let registry = registry.clone();
                let trust_store = trust_store.clone();
                let dialing = dialing.clone();
                let addresses = found.addresses.clone();
                tokio::spawn(async move {
                    let reached = registry
                        .connect_discovered(&peer.fingerprint, peer.transport_spki_hash, &addresses)
                        .await;
                    if let Some(addr) = reached {
                        if let Err(error) =
                            trust_store.set_last_endpoint(&peer.fingerprint, &addr.to_string())
                        {
                            warn!(
                                "Could not save the address of {}: {error}",
                                peer.fingerprint
                            );
                        }
                    }
                    dialing.lock().unwrap().remove(&peer.fingerprint);
                });
            }
        }

        tokio::time::sleep(RESTART_DELAY).await;
    }
}
