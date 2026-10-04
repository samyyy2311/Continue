// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;
use std::sync::Arc;

use history::{Direction, HistoryStore, Item, Kind};
use pairing::TrustStore;
use permissions::PermissionStore;

use crate::DeviceError;

/// What a device keeps across restarts, all in one database.
#[derive(Clone)]
pub struct Stores {
    pub trust: TrustStore,
    pub permissions: Arc<PermissionStore>,
    pub history: HistoryStore,
}

/// A received file or text, as saved to history.
pub struct Recorded {
    pub peer_name: String,
    /// None if history couldn't save it.
    pub history_id: Option<i64>,
}

impl Stores {
    pub fn open(db: impl AsRef<Path>) -> Result<Self, DeviceError> {
        let db = db.as_ref();
        Ok(Self {
            trust: TrustStore::open(db)?,
            permissions: Arc::new(PermissionStore::open(db)?),
            history: HistoryStore::open(db)?,
        })
    }

    /// The name a paired device goes by; empty if it never sent one.
    pub fn peer_name(&self, fingerprint: &str) -> String {
        self.trust
            .get_peer(fingerprint)
            .ok()
            .flatten()
            .map(|peer| peer.display_name)
            .unwrap_or_default()
    }

    /// Saves the name a device sent. True if it changed.
    pub fn rename_peer(&self, fingerprint: &str, name: &str) -> bool {
        self.trust
            .set_display_name(fingerprint, name)
            .map_err(|error| tracing::warn!("Couldn't save the name of {fingerprint}: {error}"))
            .unwrap_or(false)
    }

    /// Saves to history. A failure there shouldn't fail what was actually done, so it is
    /// only logged.
    pub fn remember(&self, item: Item) -> Option<i64> {
        self.history
            .record(&item)
            .map_err(|error| tracing::warn!("Couldn't save to history: {error}"))
            .ok()
    }

    pub fn received_file(
        &self,
        peer: &str,
        file: &transfer::ReceivedFile,
        location: Option<String>,
    ) -> Recorded {
        self.received(
            peer,
            Kind::File,
            file.file_name.clone(),
            file.bytes_received,
            location,
        )
    }

    pub fn received_text(&self, peer: &str, text: &str) -> Recorded {
        self.received(peer, Kind::Text, text.to_string(), text.len() as u64, None)
    }

    fn received(
        &self,
        peer: &str,
        kind: Kind,
        label: String,
        size: u64,
        location: Option<String>,
    ) -> Recorded {
        let peer_name = self.peer_name(peer);
        let history_id = self.remember(Item {
            direction: Direction::Received,
            kind,
            label,
            peer_fingerprint: peer.to_string(),
            peer_name: peer_name.clone(),
            size,
            failed: false,
            location,
        });
        Recorded {
            peer_name,
            history_id,
        }
    }
}
