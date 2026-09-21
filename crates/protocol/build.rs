// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

fn main() {
    let proto_dir = PathBuf::from("../../proto");
    let proto_files = [
        proto_dir.join("common.proto"),
        proto_dir.join("device.proto"),
        proto_dir.join("discovery.proto"),
        proto_dir.join("pairing.proto"),
        proto_dir.join("session.proto"),
        proto_dir.join("capabilities.proto"),
        proto_dir.join("permissions.proto"),
        proto_dir.join("errors.proto"),
        proto_dir.join("transfer.proto"),
        proto_dir.join("clipboard.proto"),
        proto_dir.join("notification.proto"),
    ];

    for file in &proto_files {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    prost_build::Config::new()
        .compile_protos(&proto_files, &[proto_dir])
        .expect("Failed to compile protobuf files");
}
