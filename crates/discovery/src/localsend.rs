// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

pub const LOCALSEND_MULTICAST_IPV4: &str = "224.0.0.167";
pub const LOCALSEND_DEFAULT_PORT: u16 = 53317;
pub const LOCALSEND_PROTOCOL_VERSION: &str = "2.0";

fn default_port() -> u16 {
    LOCALSEND_DEFAULT_PORT
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalSendAnnouncement {
    pub alias: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,
    pub fingerprint: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub protocol: String,
    #[serde(default)]
    pub download: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announcement: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub announce: Option<bool>,
}

impl LocalSendAnnouncement {
    pub fn new(
        alias: impl Into<String>,
        fingerprint: impl Into<String>,
        port: u16,
        is_desktop: bool,
    ) -> Self {
        Self {
            alias: alias.into(),
            version: LOCALSEND_PROTOCOL_VERSION.to_string(),
            device_model: Some(if is_desktop {
                "Desktop PC".into()
            } else {
                "Mobile Device".into()
            }),
            device_type: Some(if is_desktop {
                "desktop".into()
            } else {
                "mobile".into()
            }),
            fingerprint: fingerprint.into(),
            port,
            protocol: "https".to_string(),
            download: false,
            announcement: Some(true),
            announce: Some(true),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn announcement_roundtrip_matches_localsend_v2_spec() {
        let announcement =
            LocalSendAnnouncement::new("Continue-Desktop", "continue-fp-01", 53317, true);
        let bytes = announcement.to_bytes().expect("serialize");

        let json_str = std::str::from_utf8(&bytes).unwrap();
        assert!(json_str.contains("\"alias\":\"Continue-Desktop\""));
        assert!(json_str.contains("\"version\":\"2.0\""));
        assert!(json_str.contains("\"port\":53317"));
        assert!(json_str.contains("\"deviceType\":\"desktop\""));

        let decoded = LocalSendAnnouncement::from_bytes(&bytes).expect("deserialize");
        assert_eq!(decoded.alias, "Continue-Desktop");
        assert_eq!(decoded.fingerprint, "continue-fp-01");
        assert_eq!(decoded.port, 53317);
        assert_eq!(decoded.device_type.as_deref(), Some("desktop"));
    }

    #[test]
    fn parses_external_localsend_json_payload() {
        let external_json = r#"{
            "alias": "Alice's iPhone",
            "version": "2.0",
            "deviceModel": "iPhone 15 Pro",
            "deviceType": "mobile",
            "fingerprint": "ls-ios-8821",
            "port": 53317,
            "protocol": "https",
            "download": false,
            "announcement": true
        }"#;

        let decoded = LocalSendAnnouncement::from_bytes(external_json.as_bytes()).expect("parse");
        assert_eq!(decoded.alias, "Alice's iPhone");
        assert_eq!(decoded.device_model.as_deref(), Some("iPhone 15 Pro"));
        assert_eq!(decoded.device_type.as_deref(), Some("mobile"));
        assert_eq!(decoded.fingerprint, "ls-ios-8821");
    }
}
