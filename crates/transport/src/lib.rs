// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod cert;
pub mod endpoint;
pub mod error;
pub mod framed;
pub mod session;
pub mod spki;
pub mod verifier;

pub use cert::TransportCertificate;
pub use endpoint::{
    connect_pinned, create_client_endpoint, create_server_endpoint, default_transport_config,
    DialConfig, ALPN_CONTINUE,
};
pub use error::TransportError;
pub use framed::{read_msg, write_msg};
pub use session::{AuthenticatedPeerSession, ConnectionPath};
pub use spki::{compute_spki_hash, extract_and_hash_spki, extract_spki_bytes};
pub use verifier::{
    extract_peer_spki_hash, PairingClientCertVerifier, PinnedClientCertVerifier,
    PinnedServerCertVerifier, TrustedPeersClientCertVerifier,
};
