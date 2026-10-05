// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! The names devices show each other, exchanged when a session starts.

use std::sync::{Arc, RwLock};

use protocol::v1::{DeviceInfo, DeviceStatus, Platform};

use limits::MAX_DEVICE_NAME_BYTES as MAX_NAME_BYTES;

struct Own {
    name: String,
    platform: Platform,
    status: Option<DeviceStatus>,
}

/// How this device introduces itself. Shared, so a rename or a new battery reading reaches
/// every session.
#[derive(Clone)]
pub struct ThisDevice(Arc<RwLock<Own>>);

impl ThisDevice {
    pub fn new(name: impl Into<String>, platform: Platform) -> Self {
        Self(Arc::new(RwLock::new(Own {
            name: clean_name(&name.into()),
            platform,
            status: None,
        })))
    }

    pub fn name(&self) -> String {
        self.0.read().unwrap().name.clone()
    }

    pub fn set_name(&self, name: &str) {
        self.0.write().unwrap().name = clean_name(name);
    }

    pub(crate) fn status(&self) -> Option<DeviceStatus> {
        self.0.read().unwrap().status
    }

    /// Records a battery reading. False if it's the one peers already have.
    pub(crate) fn set_status(&self, status: DeviceStatus) -> bool {
        let mut own = self.0.write().unwrap();
        let changed = own.status != Some(status);
        own.status = Some(status);
        changed
    }

    pub(crate) fn info(&self, fingerprint: &str) -> DeviceInfo {
        let own = self.0.read().unwrap();
        let (name, platform) = (own.name.clone(), own.platform);
        DeviceInfo {
            fingerprint: fingerprint.to_string(),
            display_name: name,
            platform: platform as i32,
            protocol_version: protocol::CURRENT_PROTOCOL_VERSION,
            ed25519_public_key: Vec::new(),
        }
    }
}

impl Default for ThisDevice {
    fn default() -> Self {
        Self::new("", this_platform())
    }
}

/// What a peer said about itself, already cleaned up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerDevice {
    pub name: String,
    pub platform: Platform,
}

impl PeerDevice {
    /// `None` when the peer sent no usable name.
    pub(crate) fn from_info(info: &DeviceInfo) -> Option<Self> {
        let name = clean_name(&info.display_name);
        (!name.is_empty()).then(|| Self {
            name,
            platform: Platform::try_from(info.platform).unwrap_or(Platform::Unspecified),
        })
    }
}

/// The platform this build runs on.
pub fn this_platform() -> Platform {
    if cfg!(target_os = "android") {
        Platform::Android
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::Macos
    } else if cfg!(target_os = "ios") {
        Platform::Ios
    } else if cfg!(target_os = "linux") {
        Platform::Linux
    } else {
        Platform::Unspecified
    }
}

/// Makes a name fit to show: one line, no control characters, trimmed, and at most
/// `MAX_NAME_BYTES` long without splitting a character.
pub fn clean_name(raw: &str) -> String {
    let one_line: String = raw
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .filter(|c| !c.is_control())
        .collect();
    let mut name = String::new();
    for word in one_line.split(' ').filter(|word| !word.is_empty()) {
        let separator = usize::from(!name.is_empty());
        for (i, c) in word.chars().enumerate() {
            let extra = if i == 0 { separator } else { 0 };
            if name.len() + extra + c.len_utf8() > MAX_NAME_BYTES {
                return name;
            }
            if extra == 1 {
                name.push(' ');
            }
            name.push(c);
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_to_one_line() {
        assert_eq!(clean_name("  Sam's\tPixel \n 8 "), "Sam's Pixel 8");
        assert_eq!(clean_name("evil\u{0}\u{1b}[31mname"), "evil[31mname");
        assert_eq!(clean_name(" \n\t "), "");
    }

    #[test]
    fn long_names_are_cut_without_splitting_a_character() {
        let name = clean_name(&"é".repeat(40));
        assert_eq!(name.len(), MAX_NAME_BYTES);
        assert_eq!(name, "é".repeat(32));

        let name = clean_name(&format!("{} tail", "a".repeat(63)));
        assert_eq!(name, "a".repeat(63), "no trailing space");
    }

    #[test]
    fn a_peer_without_a_name_is_ignored() {
        let info = DeviceInfo {
            display_name: "  ".to_string(),
            ..Default::default()
        };
        assert_eq!(PeerDevice::from_info(&info), None);

        let info = DeviceInfo {
            display_name: "Work laptop".to_string(),
            platform: Platform::Linux as i32,
            ..Default::default()
        };
        assert_eq!(
            PeerDevice::from_info(&info),
            Some(PeerDevice {
                name: "Work laptop".to_string(),
                platform: Platform::Linux
            })
        );
    }
}
