// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod adb_browser;
pub mod advertiser;
pub mod browser;
pub mod doctor;
pub mod ephemeral;
pub mod error;
pub mod localsend;

pub use adb_browser::{
    AdbDiscoveryBrowser, AdbServiceType, DiscoveredAdbService, ADB_TLS_CONNECT_SERVICE,
    ADB_TLS_PAIRING_SERVICE,
};
pub use advertiser::{advertise, DiscoveryAdvertiser, RESTART_DELAY, SERVICE_TYPE};
pub use browser::{DiscoveredPeer, DiscoveryBrowser};
pub use doctor::{
    classify_ip, diagnose_network, InterfaceKind, InterfaceReport, NetworkDiagnostic, NetworkIssue,
};
pub use ephemeral::{EphemeralDiscoveryId, ROTATION_PERIOD};
pub use error::DiscoveryError;
pub use localsend::{
    LocalSendAnnouncement, LOCALSEND_DEFAULT_PORT, LOCALSEND_MULTICAST_IPV4,
    LOCALSEND_PROTOCOL_VERSION,
};
