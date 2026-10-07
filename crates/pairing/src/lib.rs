// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

pub mod error;
pub mod flow;
pub mod keys;
pub mod qr;
pub mod replay;
pub mod trust_store;
pub mod words;

pub use error::PairingError;
pub use flow::{InitiatorPairing, PendingPeer, ResponderPairing};
pub use keys::DeviceKeys;
pub use qr::{QrPayload, QR_FORMAT_VERSION, QR_ROLE_INITIATOR};
pub use replay::ReplayCache;
pub use trust_store::{TrustStore, TrustedPeer};
pub use words::{pairing_words, presence_token, PRESENCE_WINDOW_SECS};
