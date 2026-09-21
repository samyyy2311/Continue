// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use quinn::{ClientConfig as QuinnClientConfig, Endpoint, ServerConfig as QuinnServerConfig, TransportConfig};
use rustls::{ClientConfig as RustlsClientConfig, ServerConfig as RustlsServerConfig};

use crate::error::TransportError;

/// ALPN identifier for Continue protocol v1.
pub const ALPN_CONTINUE: &[&[u8]] = &[b"continue-v1"];

/// Create a default Quinn TransportConfig applying protocol limits.
pub fn default_transport_config() -> Arc<TransportConfig> {
    let mut config = TransportConfig::default();
    config.max_idle_timeout(Some(
        Duration::from_secs(limits::STREAM_IDLE_TIMEOUT_SECS)
            .try_into()
            .unwrap_or_else(|_| quinn::IdleTimeout::from(quinn::VarInt::from_u32(60))),
    ));
    config.keep_alive_interval(Some(Duration::from_secs(limits::KEEPALIVE_INTERVAL_SECS)));
    Arc::new(config)
}

/// Create a QUIC server endpoint bound to the given socket address.
pub fn create_server_endpoint(
    bind_addr: SocketAddr,
    server_tls: RustlsServerConfig,
) -> Result<Endpoint, TransportError> {
    let quic_server_config = QuicServerConfig::try_from(server_tls)
        .map_err(|e| TransportError::HandshakeFailed(format!("Failed to build QUIC server config: {e}")))?;

    let mut quinn_server_config = QuinnServerConfig::with_crypto(Arc::new(quic_server_config));
    quinn_server_config.transport_config(default_transport_config());

    let endpoint = Endpoint::server(quinn_server_config, bind_addr)?;
    Ok(endpoint)
}

/// Create a QUIC client endpoint bound to a local socket address.
pub fn create_client_endpoint(
    bind_addr: SocketAddr,
    client_tls: RustlsClientConfig,
) -> Result<Endpoint, TransportError> {
    let quic_client_config = QuicClientConfig::try_from(client_tls)
        .map_err(|e| TransportError::HandshakeFailed(format!("Failed to build QUIC client config: {e}")))?;

    let mut quinn_client_config = QuinnClientConfig::new(Arc::new(quic_client_config));
    quinn_client_config.transport_config(default_transport_config());

    let mut endpoint = Endpoint::client(bind_addr)?;
    endpoint.set_default_client_config(quinn_client_config);
    Ok(endpoint)
}
