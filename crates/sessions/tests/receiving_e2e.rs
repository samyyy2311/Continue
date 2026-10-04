// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{link, scratch_dir, send_sized};
use sessions::{IncomingEvent, IncomingFiles, SessionCapabilityHandlers};

const BIG: usize = 8 * 1024 * 1024;

/// Handlers that record what they hear about incoming files.
fn recording() -> (
    SessionCapabilityHandlers,
    IncomingFiles,
    Arc<Mutex<Vec<IncomingEvent>>>,
) {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let sink = heard.clone();
    let incoming =
        IncomingFiles::with_listener(Arc::new(move |event| sink.lock().unwrap().push(event)));
    let handlers = SessionCapabilityHandlers::new(scratch_dir()).with_incoming(incoming.clone());
    (handlers, incoming, heard)
}

/// Waits for the receiver to finish with the file, which can be just after the sender hears back.
async fn until_ended(heard: &Mutex<Vec<IncomingEvent>>) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !heard
            .lock()
            .unwrap()
            .iter()
            .any(|event| matches!(event, IncomingEvent::Ended { .. }))
        {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the receiver never finished with the file");
}

#[tokio::test]
async fn progress_is_reported_from_start_to_finish() {
    let (handlers, incoming, heard) = recording();
    let folder = handlers.save_folder.get();
    let link = link(handlers).await;

    assert!(send_sized(&link, "video.mp4", BIG).await);
    until_ended(&heard).await;

    let heard = heard.lock().unwrap();
    let progress: Vec<_> = heard
        .iter()
        .filter_map(|event| match event {
            IncomingEvent::Progress(file) => Some(file.received),
            IncomingEvent::Ended { .. } => None,
        })
        .collect();
    assert_eq!(progress.first(), Some(&0));
    assert_eq!(progress.last(), Some(&(BIG as u64)));
    // Reported per percent, not per chunk.
    assert!(progress.len() <= 102, "{} updates", progress.len());
    assert!(matches!(heard.last(), Some(IncomingEvent::Ended { .. })));
    assert!(incoming.list().is_empty());
    assert!(folder.join("video.mp4").exists());
}

#[tokio::test]
async fn cancelling_stops_the_file_and_tells_the_sender() {
    let (handlers, incoming, heard) = recording();
    let folder = handlers.save_folder.get();
    let link = link(handlers).await;

    let canceller = incoming.clone();
    let cancel = tokio::spawn(async move {
        // Cancels as soon as the file shows up as coming in.
        loop {
            if let Some(file) = canceller.list().first() {
                assert!(canceller.cancel(&file.transfer_id));
                return;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });

    assert!(!send_sized(&link, "huge.iso", 4 * BIG).await);
    cancel.await.unwrap();
    until_ended(&heard).await;
    assert!(incoming.list().is_empty());
    assert_eq!(std::fs::read_dir(folder).unwrap().count(), 0);
    assert!(!incoming.cancel("tx-huge.iso"), "nothing left to cancel");
}

#[tokio::test]
async fn a_new_save_folder_applies_to_the_next_file() {
    let (handlers, _, _) = recording();
    let save_folder = handlers.save_folder.clone();
    let link = link(handlers).await;

    let chosen = scratch_dir();
    save_folder.set(&chosen);
    assert!(send_sized(&link, "after.txt", 10).await);
    assert!(chosen.join("after.txt").exists());
}
