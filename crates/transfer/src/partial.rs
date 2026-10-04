// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

//! The partial file a transfer writes into until it is complete and verified. One for a file
//! that can resume is named after the file's checksum and size, so a later attempt at the
//! same file finds it; it survives a dropped connection and is pruned once left a day.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

use limits::TRANSFER_CHUNK_BYTES;
use protocol::v1::FileTransferRequest;

use crate::hex::hex_encode;

const PREFIX: &str = ".continue-";
const SUFFIX: &str = ".part";

/// A partial file nobody has touched for this long belongs to a transfer that won't resume.
pub const STALE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// Partial files being written right now, so two transfers of the same file never share one.
static IN_USE: Mutex<Option<HashSet<PathBuf>>> = Mutex::new(None);

/// A transfer's claim on its partial file, released when dropped.
pub struct Partial {
    pub path: PathBuf,
    /// Whether a later attempt at the same file may pick this one up.
    pub resumable: bool,
}

impl Drop for Partial {
    fn drop(&mut self) {
        if let Some(in_use) = IN_USE.lock().unwrap().as_mut() {
            in_use.remove(&self.path);
        }
    }
}

fn claim(path: PathBuf) -> Option<PathBuf> {
    let mut in_use = IN_USE.lock().unwrap();
    in_use
        .get_or_insert_with(HashSet::new)
        .insert(path.clone())
        .then_some(path)
}

/// Claims the partial file for `req` in `dir`. A resumable request gets the one shared by
/// every attempt at the same file, unless another transfer of it is under way; then, like
/// a request that can't resume, it gets one of its own.
pub fn claim_for(dir: &Path, req: &FileTransferRequest) -> Partial {
    if req.resumable && req.sha256_checksum.len() == 32 {
        let shared = dir.join(format!(
            "{PREFIX}{}-{}{SUFFIX}",
            hex_encode(&req.sha256_checksum),
            req.file_size
        ));
        if let Some(path) = claim(shared) {
            return Partial {
                path,
                resumable: true,
            };
        }
    }
    // Named from a hash, so nothing the peer sends ends up in the path.
    let own: [u8; 32] = Sha256::digest(format!("{}:{}", req.transfer_id, rand_suffix())).into();
    let path = dir.join(format!("{PREFIX}{}{SUFFIX}", hex_encode(&own[..16])));
    Partial {
        path: claim(path.clone()).unwrap_or(path),
        resumable: false,
    }
}

fn rand_suffix() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos())
}

/// How much of the file an earlier attempt left, with those bytes already hashed. Anything
/// that can't be used as-is (too long, unreadable) means starting over.
pub async fn already_received(partial: &Partial, file_size: u64) -> (u64, Sha256) {
    let mut hasher = Sha256::new();
    if !partial.resumable {
        return (0, hasher);
    }
    let Ok(mut file) = tokio::fs::File::open(&partial.path).await else {
        return (0, hasher);
    };
    let mut have = 0u64;
    let mut chunk = vec![0u8; TRANSFER_CHUNK_BYTES];
    loop {
        match file.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => {
                hasher.update(&chunk[..n]);
                have += n as u64;
            }
            Err(_) => return (0, Sha256::new()),
        }
    }
    if have > file_size {
        return (0, Sha256::new());
    }
    (have, hasher)
}

/// Removes partial files in `dir` that no transfer has touched for `STALE_AFTER`.
pub async fn prune_stale(dir: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return;
    };
    let now = SystemTime::now();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(PREFIX) || !name.ends_with(SUFFIX) {
            continue;
        }
        let path = entry.path();
        let in_use = IN_USE
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|in_use| in_use.contains(&path));
        let stale = entry
            .metadata()
            .await
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age > STALE_AFTER);
        if stale && !in_use {
            let _ = tokio::fs::remove_file(path).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(resumable: bool) -> FileTransferRequest {
        FileTransferRequest {
            transfer_id: "../../etc/passwd".to_string(),
            file_name: "a.bin".to_string(),
            file_size: 10,
            sha256_checksum: vec![7; 32],
            mime_type: String::new(),
            resumable,
        }
    }

    #[test]
    fn attempts_at_the_same_file_share_a_partial_but_never_at_once() {
        let dir = Path::new("/tmp/continue-partial-test");
        let first = claim_for(dir, &request(true));
        assert!(first.resumable);

        let concurrent = claim_for(dir, &request(true));
        assert!(!concurrent.resumable, "the shared one is taken");
        assert_ne!(concurrent.path, first.path);

        let path = first.path.clone();
        drop(first);
        let later = claim_for(dir, &request(true));
        assert_eq!(later.path, path, "a later attempt finds it");
    }

    #[test]
    fn the_peers_transfer_id_never_reaches_the_path() {
        let dir = Path::new("/tmp/continue-partial-test-ids");
        let own = claim_for(dir, &request(false));
        assert_eq!(own.path.parent(), Some(dir));
        let name = own.path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with(PREFIX) && !name.contains(".."), "{name}");
    }
}
