// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::error::TransferError;
use crate::sanitizer::sanitize_filename;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalSendSenderInfo {
    pub alias: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalSendFileMetadata {
    pub id: String,
    pub file_name: String,
    pub size: u64,
    pub file_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalSendPrepareRequest {
    pub info: LocalSendSenderInfo,
    pub files: HashMap<String, LocalSendFileMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalSendPrepareResponse {
    pub session_id: String,
    pub files: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct LocalSendSanitizedFile {
    pub id: String,
    pub sanitized_name: String,
    pub size: u64,
    pub file_type: String,
    pub token: String,
    pub completed: bool,
}

#[derive(Debug, Clone)]
pub struct LocalSendActiveSession {
    pub session_id: String,
    pub sender: LocalSendSenderInfo,
    pub files: HashMap<String, LocalSendSanitizedFile>,
}

#[derive(Default)]
pub struct LocalSendBridgeSessionManager {
    sessions: HashMap<String, LocalSendActiveSession>,
}

impl LocalSendBridgeSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prepare_upload(
        &mut self,
        req: LocalSendPrepareRequest,
    ) -> Result<LocalSendPrepareResponse, TransferError> {
        let session_id = format!("ls-sess-{}", rand::random::<u64>());
        let mut response_files = HashMap::new();
        let mut session_files = HashMap::new();

        for (file_id, meta) in req.files {
            let sanitized_name = sanitize_filename(&meta.file_name)?;
            let token = format!("tok-{}", rand::random::<u64>());

            response_files.insert(file_id.clone(), token.clone());
            session_files.insert(
                file_id.clone(),
                LocalSendSanitizedFile {
                    id: file_id,
                    sanitized_name,
                    size: meta.size,
                    file_type: meta.file_type,
                    token,
                    completed: false,
                },
            );
        }

        let session = LocalSendActiveSession {
            session_id: session_id.clone(),
            sender: req.info,
            files: session_files,
        };

        self.sessions.insert(session_id.clone(), session);

        Ok(LocalSendPrepareResponse {
            session_id,
            files: response_files,
        })
    }

    pub fn verify_upload(
        &self,
        session_id: &str,
        file_id: &str,
        token: &str,
    ) -> Result<&LocalSendSanitizedFile, TransferError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| TransferError::BridgeSession("Session not found".into()))?;

        let file = session
            .files
            .get(file_id)
            .ok_or_else(|| TransferError::BridgeSession("File ID not found in session".into()))?;

        if file.token != token {
            return Err(TransferError::BridgeSession("Invalid upload token".into()));
        }

        Ok(file)
    }

    pub fn mark_completed(
        &mut self,
        session_id: &str,
        file_id: &str,
    ) -> Result<(), TransferError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| TransferError::BridgeSession("Session not found".into()))?;

        let file = session
            .files
            .get_mut(file_id)
            .ok_or_else(|| TransferError::BridgeSession("File ID not found in session".into()))?;

        file.completed = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_upload_sanitizes_filenames_and_issues_tokens() {
        let mut manager = LocalSendBridgeSessionManager::new();

        let mut files = HashMap::new();
        files.insert(
            "f1".into(),
            LocalSendFileMetadata {
                id: "f1".into(),
                file_name: "../../../evil/payload.sh".into(),
                size: 1024,
                file_type: "text/x-sh".into(),
                sha256: None,
            },
        );

        let req = LocalSendPrepareRequest {
            info: LocalSendSenderInfo {
                alias: "iPhone".into(),
                version: "2.0".into(),
                device_model: Some("iPhone 15".into()),
                device_type: Some("mobile".into()),
                fingerprint: "ls-iphone-1".into(),
            },
            files,
        };

        let resp = manager.prepare_upload(req).expect("prepare success");
        assert!(resp.files.contains_key("f1"));
        let token = resp.files.get("f1").unwrap();

        let verified = manager.verify_upload(&resp.session_id, "f1", token).unwrap();
        // Filename traversal components stripped to safe basename
        assert_eq!(verified.sanitized_name, "payload.sh");
        assert_eq!(verified.size, 1024);

        // Verification fails with invalid token
        assert!(manager.verify_upload(&resp.session_id, "f1", "bad-tok").is_err());
    }

    #[test]
    fn marks_file_upload_completed() {
        let mut manager = LocalSendBridgeSessionManager::new();

        let mut files = HashMap::new();
        files.insert(
            "img-01".into(),
            LocalSendFileMetadata {
                id: "img-01".into(),
                file_name: "photo.jpg".into(),
                size: 2048,
                file_type: "image/jpeg".into(),
                sha256: None,
            },
        );

        let req = LocalSendPrepareRequest {
            info: LocalSendSenderInfo {
                alias: "MacBook".into(),
                version: "2.0".into(),
                device_model: Some("MacBook Pro".into()),
                device_type: Some("desktop".into()),
                fingerprint: "ls-mac-1".into(),
            },
            files,
        };

        let resp = manager.prepare_upload(req).unwrap();
        manager.mark_completed(&resp.session_id, "img-01").unwrap();

        let verified = manager
            .verify_upload(&resp.session_id, "img-01", resp.files.get("img-01").unwrap())
            .unwrap();
        assert!(verified.completed);
    }
}
