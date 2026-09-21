// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: GPL-3.0

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DeviceIdentityDto {
    pub device_name: String,
    pub fingerprint: String,
    pub spki_hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct TrustedPeerDto {
    pub fingerprint: String,
    pub display_name: String,
    pub paired_at: u64,
    pub is_connected: bool,
}

#[tauri::command]
fn get_device_identity() -> Result<DeviceIdentityDto, String> {
    Ok(DeviceIdentityDto {
        device_name: "Desktop PC".to_string(),
        fingerprint: "cont1q8f7e2a9d4c6b8a1e3f5a7b9c1d3e5f7a9b1c3d".to_string(),
        spki_hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
    })
}

#[tauri::command]
fn get_trusted_peers() -> Result<Vec<TrustedPeerDto>, String> {
    Ok(vec![])
}

#[tauri::command]
fn start_pairing(listen_port: u16, advertised_endpoint: String) -> Result<String, String> {
    Ok(format!(
        "continue://pair/v1?port={listen_port}&ep={advertised_endpoint}"
    ))
}

#[tauri::command]
fn pair_from_qr(qr_payload: String) -> Result<TrustedPeerDto, String> {
    if qr_payload.is_empty() {
        return Err("QR payload is empty".to_string());
    }
    Ok(TrustedPeerDto {
        fingerprint: "cont1q_paired_mock_device".to_string(),
        display_name: "Paired Device".to_string(),
        paired_at: 1726920000,
        is_connected: true,
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_device_identity,
            get_trusted_peers,
            start_pairing,
            pair_from_qr
        ])
        .run(tauri::generate_context!())
        .expect("error while running Continue desktop application");
}
