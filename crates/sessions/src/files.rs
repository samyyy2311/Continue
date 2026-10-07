// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use limits::MAX_FRAME_FILES_BYTES;
use protocol::v1::{files_message, FileEntry, FilesMessage, FilesReply};
use protocol::CapabilityId;
use transport::{read_msg, write_msg, TransportError};

use crate::capabilities_router::{
    allowed, off_runtime, send_requested_file, SessionCapabilityHandlers,
};
use crate::multiplexer::{IncomingCapabilityStream, SessionMultiplexer};

/// Most entries one folder's listing holds.
pub const MAX_ENTRIES: usize = 2000;

/// A requested file follows as a normal file transfer.
pub(crate) async fn serve_files(
    mux: &SessionMultiplexer,
    handlers: &SessionCapabilityHandlers,
    peer: &str,
    mut stream: IncomingCapabilityStream,
) -> Result<(), TransportError> {
    let message: FilesMessage = read_msg(&mut stream.recv_stream, MAX_FRAME_FILES_BYTES).await?;
    let mut reply = FilesReply::default();
    let mut wanted = None;
    match (message.body, handlers.shared_folder.clone()) {
        (Some(body), Some(root)) if allowed(handlers, peer, CapabilityId::FILES) => match body {
            files_message::Body::List(list) => {
                let entries = off_runtime(move || list_folder(&root, &list.path)).await;
                reply.available = entries.is_some();
                reply.entries = entries.unwrap_or_default();
            }
            files_message::Body::Get(get) => {
                wanted =
                    off_runtime(move || inside(&root, &get.path).filter(|p| p.is_file())).await;
                reply.available = wanted.is_some();
            }
        },
        _ => {}
    }
    write_msg(&mut stream.send_stream, &reply, MAX_FRAME_FILES_BYTES).await?;
    let _ = stream.send_stream.finish();

    if let Some(path) = wanted {
        send_requested_file(mux, peer, &path).await;
    }
    Ok(())
}

/// Where `relative` is under `root`, or None if it would lead outside it, by `..` or a link.
fn inside(root: &Path, relative: &str) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    for part in Path::new(relative).components() {
        match part {
            Component::Normal(name) => path.push(name),
            Component::CurDir => {}
            _ => return None,
        }
    }
    let path = path.canonicalize().ok()?;
    path.starts_with(root.canonicalize().ok()?).then_some(path)
}

/// Folders first, then files, each by name, leaving out hidden ones.
fn list_folder(root: &Path, relative: &str) -> Option<Vec<FileEntry>> {
    let folder = inside(root, relative)?;
    let mut entries: Vec<FileEntry> = std::fs::read_dir(folder)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
            Some(FileEntry {
                folder: metadata.is_dir(),
                size: if metadata.is_dir() { 0 } else { metadata.len() },
                modified: modified.as_millis() as u64,
                name,
            })
            .filter(|entry| !entry.name.starts_with('.'))
        })
        .take(MAX_ENTRIES)
        .collect();
    entries.sort_by(|a, b| {
        b.folder
            .cmp(&a.folder)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Some(entries)
}

impl SessionMultiplexer {
    pub async fn send_files_message(
        &self,
        body: files_message::Body,
    ) -> Result<FilesReply, TransportError> {
        let message = FilesMessage { body: Some(body) };
        self.ask(CapabilityId::FILES, &message, MAX_FRAME_FILES_BYTES)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_stay_inside_the_shared_folder() {
        let root = std::env::temp_dir().join(format!("continue-files-{}", rand::random::<u64>()));
        std::fs::create_dir_all(root.join("DCIM")).unwrap();
        std::fs::write(root.join("DCIM/beach.jpg"), b"photo").unwrap();

        assert!(inside(&root, "DCIM/beach.jpg").is_some());
        assert!(inside(&root, "").is_some());
        assert!(inside(&root, "../").is_none());
        assert!(inside(&root, "DCIM/../../etc").is_none());
        assert!(inside(&root, "/etc").is_none());

        let listed = list_folder(&root, "").unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].folder);
        std::fs::remove_dir_all(root).unwrap();
    }
}
