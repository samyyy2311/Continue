// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, RwLock};

use pairing::TrustStore;
use tracing::{debug, warn};
use transport::{TransportCertificate, TransportError};

use crate::registry::{Direction, SessionRegistry};

/// Paired devices find each other here across restarts. If another program holds it,
/// the listener falls back to a random port for that run.
pub const DEFAULT_LISTEN_PORT: u16 = 47470;

/// Saves where a paired device can be dialed: its IP with the default listening port.
// If the peer fell back to a random port, discovery or a manual connect finds it instead.
pub fn remember_peer_address(trust_store: &TrustStore, fingerprint: &str, ip: IpAddr) {
    let endpoint = SocketAddr::new(ip, DEFAULT_LISTEN_PORT).to_string();
    if let Err(error) = trust_store.set_last_endpoint(fingerprint, &endpoint) {
        warn!("Could not save the address of {fingerprint}: {error}");
    }
}

/// Opens the endpoint paired devices connect to. The TLS handshake only completes for
/// clients whose transport key is in `trusted_keys`.
pub fn listen_for_peers(
    cert: &TransportCertificate,
    trusted_keys: Arc<RwLock<HashSet<[u8; 32]>>>,
) -> Result<quinn::Endpoint, TransportError> {
    let server_tls = cert.build_trusted_peers_server_tls(trusted_keys)?;
    let bind = |port: u16| {
        transport::create_server_endpoint(
            SocketAddr::from(([0, 0, 0, 0], port)),
            server_tls.clone(),
        )
    };
    bind(DEFAULT_LISTEN_PORT).or_else(|_| bind(0))
}

/// Adds each incoming connection from a paired device to `registry`, until `endpoint`
/// is closed. Every handshake runs on its own task, so a slow one doesn't hold up the rest.
pub async fn accept_peers(
    endpoint: quinn::Endpoint,
    registry: SessionRegistry,
    trust_store: TrustStore,
) {
    while let Some(incoming) = endpoint.accept().await {
        let registry = registry.clone();
        let trust_store = trust_store.clone();
        tokio::spawn(async move {
            let connection = match incoming.await {
                Ok(connection) => connection,
                Err(error) => {
                    debug!("Incoming handshake failed: {error}");
                    return;
                }
            };
            // The TLS layer already checked the key; the trust store is checked again in case
            // the device was unpaired while it was connecting.
            let spki_hash = transport::extract_peer_spki_hash(&connection);
            let peer =
                spki_hash.and_then(|hash| trust_store.get_peer_by_spki_hash(&hash).ok().flatten());
            match (spki_hash, peer) {
                (Some(spki_hash), Some(peer)) => {
                    remember_peer_address(
                        &trust_store,
                        &peer.fingerprint,
                        connection.remote_address().ip(),
                    );
                    registry.attach(&peer.fingerprint, spki_hash, connection, Direction::Inbound);
                }
                _ => connection.close(0u32.into(), b"untrusted_peer"),
            }
        });
    }
}
