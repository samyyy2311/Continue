// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// Stored in the database. Persists across restarts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistedGrant {
    Allow,
    Deny,
    Ask,
}

impl PersistedGrant {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "Allow",
            Self::Deny => "Deny",
            Self::Ask => "Ask",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Allow" => Some(Self::Allow),
            "Deny" => Some(Self::Deny),
            "Ask" => Some(Self::Ask),
            _ => None,
        }
    }
}

/// Full runtime state. `AllowOnce` exists only in memory and is never persisted to disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionState {
    Allow,
    Deny,
    Ask,
    AllowOnce,
}

impl From<PersistedGrant> for PermissionState {
    fn from(grant: PersistedGrant) -> Self {
        match grant {
            PersistedGrant::Allow => Self::Allow,
            PersistedGrant::Deny => Self::Deny,
            PersistedGrant::Ask => Self::Ask,
        }
    }
}

impl PermissionState {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow | Self::AllowOnce)
    }
}
