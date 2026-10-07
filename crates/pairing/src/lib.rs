// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod adb_pairing;
pub mod error;
pub mod flow;
pub mod keys;
pub mod qr;
pub mod replay;
pub mod trust_store;
pub mod words;

pub use adb_pairing::{pair_client, pair_server, AdbPeerCredentials};
pub use error::PairingError;
pub use flow::{InitiatorPairing, ResponderPairing};
pub use keys::DeviceKeys;
pub use qr::{QrPayload, QR_FORMAT_VERSION, QR_ROLE_INITIATOR};
pub use replay::ReplayCache;
pub use trust_store::{TrustStore, TrustedPeer};
pub use words::{derive_verification_words, format_verification_phrase, WORDLIST};
