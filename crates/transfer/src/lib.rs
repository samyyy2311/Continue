// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Streaming file transfer, one QUIC stream per file.

pub mod capture;
pub mod drop_folder;
pub mod error;
pub mod localsend_bridge;
mod partial;
pub mod queue;
pub mod receiver;
pub mod sanitizer;
pub mod sender;

pub use capture::{
    format_as_clipboard_dibv5, request_camera_capture, respond_camera_capture, CameraCaptureSource,
    CapturedMedia,
};
pub use drop_folder::{guess_mime_type, is_ignorable_file, DropFolderConfig, DropFolderWatcher};
pub use error::TransferError;
pub use localsend_bridge::{
    LocalSendActiveSession, LocalSendBridgeSessionManager, LocalSendFileMetadata,
    LocalSendPrepareRequest, LocalSendPrepareResponse, LocalSendSanitizedFile, LocalSendSenderInfo,
};
pub use queue::{OfflineTransferQueue, QueuedPayload, QueuedTransfer};
pub use receiver::{receive_file, ReceivedFile};
pub use sanitizer::sanitize_filename;
pub use sender::{compute_file_sha256, send_file};
