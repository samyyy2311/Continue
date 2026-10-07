// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;
use transport::{create_client_endpoint, create_server_endpoint, TransportCertificate};

use protocol::v1::{
    CameraCaptureDestination, CameraCaptureMode, CameraCaptureRequest,
};
use transfer::{
    format_as_clipboard_dibv5, request_camera_capture, respond_camera_capture,
    CameraCaptureSource, TransferError,
};

#[tokio::test]
async fn camera_capture_e2e_photo_success() {
    let temp_dir = std::env::temp_dir().join(format!("continue_capture_test_{}", rand::random::<u32>()));
    let phone_dir = temp_dir.join("phone");
    let pc_dir = temp_dir.join("pc");
    tokio::fs::create_dir_all(&phone_dir).await.unwrap();
    tokio::fs::create_dir_all(&pc_dir).await.unwrap();

    let captured_image = phone_dir.join("IMG_20261008_013000.jpg");
    let image_data = vec![0xEEu8; 96 * 1024];
    tokio::fs::write(&captured_image, &image_data).await.unwrap();

    let server_cert = TransportCertificate::generate().unwrap();
    let client_cert = TransportCertificate::generate().unwrap();

    let server_tls = server_cert
        .build_pinned_server_tls(client_cert.spki_hash)
        .unwrap();
    let client_tls = client_cert
        .build_pinned_client_tls(server_cert.spki_hash)
        .unwrap();

    let server_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server_endpoint = create_server_endpoint(server_addr, server_tls).unwrap();
    let bound_addr = server_endpoint.local_addr().unwrap();

    let client_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let client_endpoint = create_client_endpoint(client_addr, client_tls).unwrap();

    let image_path_clone = captured_image.clone();
    let phone_handle = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.expect("accept");
        let conn = incoming.await.expect("connect");
        let (mut send_stream, mut recv_stream) = conn.accept_bi().await.expect("bi stream");

        let result = respond_camera_capture(
            &mut send_stream,
            &mut recv_stream,
            |_req: CameraCaptureRequest| async move {
                Ok(CameraCaptureSource {
                    file_path: image_path_clone,
                    mime_type: "image/jpeg".to_string(),
                    width: 1920,
                    height: 1080,
                })
            },
        )
        .await;

        (result, conn)
    });

    let client_conn = client_endpoint
        .connect(bound_addr, "continue-device")
        .unwrap()
        .await
        .unwrap();
    let (mut send_stream, mut recv_stream) = client_conn.open_bi().await.unwrap();

    let req = CameraCaptureRequest {
        capture_id: "req-capture-42".to_string(),
        mode: CameraCaptureMode::Photo as i32,
        destination: CameraCaptureDestination::Both as i32,
        flash_enabled: false,
        max_width: 1920,
        max_height: 1080,
    };

    let captured = request_camera_capture(&mut send_stream, &mut recv_stream, &pc_dir, req)
        .await
        .expect("capture success");

    let (phone_result, _phone_conn) = phone_handle.await.unwrap();
    phone_result.expect("phone success");

    assert_eq!(captured.capture_id, "req-capture-42");
    assert_eq!(captured.mode, CameraCaptureMode::Photo);
    assert_eq!(captured.destination, CameraCaptureDestination::Both);
    assert_eq!(captured.mime_type, "image/jpeg");
    assert_eq!(captured.bytes_received, image_data.len() as u64);
    assert_eq!(captured.width, 1920);
    assert_eq!(captured.height, 1080);
    assert!(captured.path.exists());

    let saved_bytes = tokio::fs::read(&captured.path).await.unwrap();
    assert_eq!(saved_bytes, image_data);

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
}

#[tokio::test]
async fn camera_capture_handles_remote_busy() {
    let temp_dir = std::env::temp_dir().join(format!("continue_capture_test_{}", rand::random::<u32>()));
    tokio::fs::create_dir_all(&temp_dir).await.unwrap();

    let server_cert = TransportCertificate::generate().unwrap();
    let client_cert = TransportCertificate::generate().unwrap();

    let server_tls = server_cert
        .build_pinned_server_tls(client_cert.spki_hash)
        .unwrap();
    let client_tls = client_cert
        .build_pinned_client_tls(server_cert.spki_hash)
        .unwrap();

    let server_endpoint = create_server_endpoint("127.0.0.1:0".parse().unwrap(), server_tls).unwrap();
    let bound_addr = server_endpoint.local_addr().unwrap();

    let client_endpoint = create_client_endpoint("127.0.0.1:0".parse().unwrap(), client_tls).unwrap();

    let phone_handle = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.expect("accept");
        let conn = incoming.await.expect("connect");
        let (mut send_stream, mut recv_stream) = conn.accept_bi().await.expect("bi stream");

        let result = respond_camera_capture(
            &mut send_stream,
            &mut recv_stream,
            |_req: CameraCaptureRequest| async move {
                Err(TransferError::CaptureBusy)
            },
        )
        .await;

        (result, conn)
    });

    let client_conn = client_endpoint
        .connect(bound_addr, "continue-device")
        .unwrap()
        .await
        .unwrap();
    let (mut send_stream, mut recv_stream) = client_conn.open_bi().await.unwrap();

    let req = CameraCaptureRequest {
        capture_id: "busy-req-1".to_string(),
        mode: CameraCaptureMode::Photo as i32,
        destination: CameraCaptureDestination::SaveFile as i32,
        flash_enabled: false,
        max_width: 0,
        max_height: 0,
    };

    let err = request_camera_capture(&mut send_stream, &mut recv_stream, &temp_dir, req)
        .await
        .expect_err("should fail with busy");

    assert!(matches!(err, TransferError::CaptureBusy));

    let (phone_result, _conn) = phone_handle.await.unwrap();
    assert!(matches!(phone_result, Err(TransferError::CaptureBusy)));

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
}

#[test]
fn dibv5_conversion_matches_windows_format() {
    let width = 4;
    let height = 3;
    let raw_bgra = vec![0x7F; 4 * 3 * 4];

    let dib = format_as_clipboard_dibv5(width, height, &raw_bgra).expect("formats dibv5");
    assert_eq!(dib.len(), 124 + raw_bgra.len());
}
