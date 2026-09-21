// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! Protocol constants, resource limits, and policy values.
//!
//! No magic numbers elsewhere in the codebase. Every protocol-level bound,
//! timeout, or size constraint is defined here and referenced by name.

// â”€â”€ Identity â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Raw SHA-256 output length in bytes.
///
/// Used for transport SPKI hashes and other SHA-256 digest fields.
/// Do not use this constant to describe the 16-byte pairing session token.
pub const SHA256_DIGEST_LEN: usize = 32;

/// Length of the Base64Url-no-padding device fingerprint string.
///
/// Computed as `Base64Url_NoPad(SHA-256(ed25519_public_key))`.
/// 32 raw bytes â†’ 43 base64url characters.
pub const DEVICE_FINGERPRINT_STR_LEN: usize = 43;

/// Maximum UTF-8 byte length of a device display name.
pub const MAX_DEVICE_NAME_BYTES: usize = 64;

// â”€â”€ Pairing â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Byte length of the pairing session token.
///
/// Distinct from SHA256_DIGEST_LEN â€” this is 16 bytes, not 32.
pub const SESSION_TOKEN_LEN: usize = 16;

/// Maximum active pairing session lifetime in seconds.
pub const MAX_PAIRING_SESSION_SECS: u64 = 60;

/// Maximum number of concurrent unexpired pairing sessions held in the replay cache.
///
/// Caps memory use from concurrent pairing attempts; excess requests are rejected.
pub const MAX_PAIRING_CACHE_ENTRIES: usize = 64;

// â”€â”€ Capabilities â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Maximum number of capability entries in a single advertisement message.
pub const MAX_CAPABILITY_ENTRIES: usize = 128;

// â”€â”€ Frame sizes (Protobuf control messages only) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Discovery and pre-session control messages.
pub const MAX_FRAME_DISCOVERY_BYTES: usize = 4 * 1024;

/// Pairing protocol messages.
pub const MAX_FRAME_PAIRING_BYTES: usize = 64 * 1024;

/// Session control messages (Ping, Pong, Disconnect, SessionEnvelope).
pub const MAX_FRAME_SESSION_BYTES: usize = 256 * 1024;

/// Capability advertisement messages.
pub const MAX_FRAME_CAPABILITY_BYTES: usize = 64 * 1024;

/// Permission negotiation messages.
pub const MAX_FRAME_PERMISSION_BYTES: usize = 64 * 1024;

/// Transfer metadata messages (not file payload bytes).
pub const MAX_FRAME_TRANSFER_META_BYTES: usize = 64 * 1024;

/// Clipboard content messages.
pub const MAX_FRAME_CLIPBOARD_BYTES: usize = 1024 * 1024;

/// Notification messages.
pub const MAX_FRAME_NOTIFICATION_BYTES: usize = 64 * 1024;

// â”€â”€ File transfer â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Maximum UTF-8 byte length of a transfer filename.
pub const MAX_TRANSFER_FILENAME_BYTES: usize = 512;

/// Raw file data chunk size for streaming transfers.
///
/// File bytes are not Protobuf-framed. This is the I/O buffer size,
/// not a Protobuf message size limit.
pub const TRANSFER_CHUNK_BYTES: usize = 64 * 1024;

/// Maximum number of concurrent file transfers.
pub const MAX_CONCURRENT_TRANSFERS: usize = 8;

// â”€â”€ Session â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Maximum queued requests before backpressure is applied.
pub const MAX_PENDING_REQUESTS: usize = 32;

// â”€â”€ Timeouts â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// QUIC connection establishment timeout in seconds.
pub const CONNECTION_TIMEOUT_SECS: u64 = 10;

/// Keepalive Ping interval in seconds.
pub const KEEPALIVE_INTERVAL_SECS: u64 = 30;

/// Keepalive timeout: if no Pong is received within this many seconds, the
/// session is considered dead.
pub const KEEPALIVE_TIMEOUT_SECS: u64 = 90;

/// Idle stream timeout in seconds. Incomplete frames are dropped after this.
pub const STREAM_IDLE_TIMEOUT_SECS: u64 = 60;

// â”€â”€ Reconnect backoff â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Initial reconnect delay in milliseconds.
pub const RETRY_INITIAL_DELAY_MS: u64 = 1_000;

/// Maximum reconnect delay in milliseconds (truncated exponential backoff).
pub const RETRY_MAX_DELAY_MS: u64 = 60_000;

/// Maximum number of reconnect attempts before giving up.
pub const RETRY_MAX_ATTEMPTS: u32 = 8;

/// Jitter fraction applied to each backoff interval (Â±25%).
pub const RETRY_JITTER_FACTOR: f64 = 0.25;

// â”€â”€ Notification content â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Maximum UTF-8 byte length of a notification body.
pub const MAX_NOTIFICATION_BODY_BYTES: usize = 4_096;
