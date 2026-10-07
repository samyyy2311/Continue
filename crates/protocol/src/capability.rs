// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

/// Type-safe capability identifier wrapping a 32-bit integer.
///
/// Unknown capability IDs received from peers are preserved without rejection
/// to ensure forward compatibility across versions.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilityId(pub u32);

impl CapabilityId {
    pub const CONTROL: Self = Self(0);
    pub const FILE_TRANSFER: Self = Self(1);
    pub const CLIPBOARD: Self = Self(2);
    pub const NOTIFICATIONS: Self = Self(3);
    pub const PHOTOS: Self = Self(4);
    pub const MESSAGES: Self = Self(5);
    pub const FILES: Self = Self(6);
    pub const CALLS: Self = Self(7);
    pub const SCREEN: Self = Self(8);
    pub const CAMERA: Self = Self(9);
    pub const MEDIA: Self = Self(10);
    pub const FIND: Self = Self(11);
    pub const POINTER: Self = Self(12);
    pub const ACTIONS: Self = Self(13);
    pub const SNIPPETS: Self = Self(14);
    pub const SEARCH: Self = Self(15);

    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }

    pub fn known_name(&self) -> Option<&'static str> {
        match *self {
            Self::CONTROL => Some("control"),
            Self::FILE_TRANSFER => Some("file_transfer"),
            Self::CLIPBOARD => Some("clipboard"),
            Self::NOTIFICATIONS => Some("notifications"),
            Self::PHOTOS => Some("photos"),
            Self::MESSAGES => Some("messages"),
            Self::FILES => Some("files"),
            Self::CALLS => Some("calls"),
            Self::SCREEN => Some("screen"),
            Self::CAMERA => Some("camera"),
            Self::MEDIA => Some("media"),
            Self::FIND => Some("find"),
            Self::POINTER => Some("pointer"),
            Self::ACTIONS => Some("actions"),
            Self::SNIPPETS => Some("snippets"),
            Self::SEARCH => Some("search"),
            _ => None,
        }
    }
}

impl From<u32> for CapabilityId {
    fn from(val: u32) -> Self {
        Self(val)
    }
}

impl From<CapabilityId> for u32 {
    fn from(id: CapabilityId) -> Self {
        id.0
    }
}

impl fmt::Debug for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.known_name() {
            write!(f, "CapabilityId({}: {})", self.0, name)
        } else {
            write!(f, "CapabilityId({})", self.0)
        }
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = self.known_name() {
            write!(f, "{}", name)
        } else {
            write!(f, "capability_{}", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_and_constants() {
        assert_eq!(CapabilityId::from(1), CapabilityId::FILE_TRANSFER);
        assert_eq!(u32::from(CapabilityId::CLIPBOARD), 2);
        assert_eq!(CapabilityId::new(3), CapabilityId::NOTIFICATIONS);

        let unknown = CapabilityId::from(999);
        assert_eq!(unknown.raw(), 999);
        assert_eq!(unknown.known_name(), None);
        assert_eq!(format!("{}", unknown), "capability_999");
    }
}
