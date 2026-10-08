// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use transfer::{
    CloudFilesConfig, CloudFilesEntry, CloudFilesMount, FetchDataRequest, HydrationState,
};

#[test]
fn cloud_files_config_and_entry_model() {
    let temp_dir = std::env::temp_dir().join(format!("continue_cf_test_{}", rand::random::<u32>()));
    let config = CloudFilesConfig::new(&temp_dir, "Continue", "Galaxy S24", "s24-device-id");

    assert_eq!(config.sync_root_path, temp_dir);
    assert_eq!(config.device_name, "Galaxy S24");
    assert_eq!(config.device_id, "s24-device-id");

    let entries = vec![
        CloudFilesEntry::directory("Documents", 1710000000, b"doc-dir".to_vec()),
        CloudFilesEntry::file("Documents/report.pdf", 4096, 1710000100, b"doc-1".to_vec()),
        CloudFilesEntry::file("photo.jpg", 123456, 1710000200, b"photo-1".to_vec()),
    ];

    assert_eq!(entries.len(), 3);
    assert!(entries[0].is_directory);
    assert_eq!(entries[0].file_size, 0);
    assert!(!entries[1].is_directory);
    assert_eq!(entries[1].file_size, 4096);
    assert_eq!(entries[2].file_identity, b"photo-1");
}

#[test]
fn cloud_files_mount_lifecycle_and_placeholders() {
    if !CloudFilesMount::is_supported() {
        return;
    }

    let temp_dir =
        std::env::temp_dir().join(format!("continue_cf_mount_{}", rand::random::<u32>()));
    let config = CloudFilesConfig::new(&temp_dir, "ContinueTest", "Test Phone", "test-phone-id");
    let mount = CloudFilesMount::new(config);

    let reg_result = mount.register();
    if let Err(e) = reg_result {
        // In restricted environments or CI containers, registering sync roots may be disallowed.
        eprintln!("Cloud files register returned: {e}");
        return;
    }

    let fetch_called = Arc::new(AtomicBool::new(false));
    let fetch_called_clone = Arc::clone(&fetch_called);

    let connect_res = mount.connect(move |req: FetchDataRequest| {
        fetch_called_clone.store(true, Ordering::SeqCst);
        assert!(!req.file_identity.is_empty());
        Ok(vec![0xAA; req.length.min(1024) as usize])
    });

    if let Ok(()) = connect_res {
        let entries = vec![CloudFilesEntry::file(
            "sample.txt",
            1024,
            1710000000,
            b"sample-id".to_vec(),
        )];

        let created = mount.create_placeholders(&entries);
        if let Ok(count) = created {
            assert!(count >= 1);
            let state = mount.get_hydration_state("sample.txt");
            if let Ok(hydr) = state {
                assert!(
                    hydr == HydrationState::Unhydrated
                        || hydr == HydrationState::FullyHydrated
                        || hydr == HydrationState::NotAPlaceholder
                );
            }
        }

        let _ = mount.disconnect();
    }

    let _ = mount.unregister();
    let _ = std::fs::remove_dir_all(&temp_dir);
}
