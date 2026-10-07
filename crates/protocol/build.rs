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
        proto_dir.join("media_capture.proto"),
        proto_dir.join("media_control.proto"),
        proto_dir.join("handoff.proto"),
        proto_dir.join("telemetry.proto"),
    ];

    for file in &proto_files {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    if std::env::var("PROTOC").is_err() {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let winget_protoc = PathBuf::from(local_app_data)
                .join(r"Microsoft\WinGet\Packages\Google.Protobuf_Microsoft.Winget.Source_8wekyb3d8bbwe\bin\protoc.exe");
            if winget_protoc.exists() {
                std::env::set_var("PROTOC", winget_protoc);
            }
        }
    }

    prost_build::Config::new()
        .compile_protos(&proto_files, &[proto_dir])
        .expect("Failed to compile protobuf files");
}
