// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use hkdf::Hkdf;
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::PairingError;

pub const ADB_PACKET_TYPE_SPAKE2: u8 = 0;
pub const ADB_PACKET_TYPE_PEER_INFO: u8 = 1;
pub const MAX_ADB_PACKET_PAYLOAD: usize = 64 * 1024;

const TOKEN_LEN: usize = 32;
const TAG_LEN: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdbPeerCredentials {
    pub name: String,
    pub key: Vec<u8>,
}

pub async fn write_adb_packet<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    packet_type: u8,
    payload: &[u8],
) -> Result<(), PairingError> {
    if payload.len() > MAX_ADB_PACKET_PAYLOAD {
        return Err(PairingError::AdbPairingFailed(format!(
            "Payload length {} exceeds limit {}",
            payload.len(),
            MAX_ADB_PACKET_PAYLOAD
        )));
    }

    writer.write_u8(packet_type).await?;
    writer.write_u32(payload.len() as u32).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_adb_packet<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<(u8, Vec<u8>), PairingError> {
    let packet_type = reader.read_u8().await?;
    let len = reader.read_u32().await? as usize;

    if len > MAX_ADB_PACKET_PAYLOAD {
        return Err(PairingError::AdbPairingFailed(format!(
            "Payload length {} exceeds limit {}",
            len, MAX_ADB_PACKET_PAYLOAD
        )));
    }

    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;
    Ok((packet_type, payload))
}

fn derive_keys(
    pin: &str,
    client_token: &[u8; TOKEN_LEN],
    server_token: &[u8; TOKEN_LEN],
) -> Result<([u8; 32], [u8; 32]), PairingError> {
    let mut salt = [0u8; TOKEN_LEN * 2];
    salt[..TOKEN_LEN].copy_from_slice(client_token);
    salt[TOKEN_LEN..].copy_from_slice(server_token);

    let hk = Hkdf::<Sha256>::new(Some(&salt), pin.as_bytes());

    let mut client_auth = [0u8; 32];
    let mut server_auth = [0u8; 32];
    hk.expand(b"adb-tls-pairing-client-auth", &mut client_auth)
        .map_err(|e| PairingError::AdbPairingFailed(e.to_string()))?;
    hk.expand(b"adb-tls-pairing-server-auth", &mut server_auth)
        .map_err(|e| PairingError::AdbPairingFailed(e.to_string()))?;

    Ok((client_auth, server_auth))
}

pub async fn pair_client<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    stream: &mut S,
    pin: &str,
    client_name: &str,
    client_key: &[u8],
) -> Result<AdbPeerCredentials, PairingError> {
    if pin.len() != 6 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(PairingError::AdbPairingFailed(
            "PIN must be a 6-digit numeric string".into(),
        ));
    }

    let mut client_token = [0u8; TOKEN_LEN];
    OsRng.fill_bytes(&mut client_token);

    let mut initial_payload = Vec::with_capacity(TOKEN_LEN);
    initial_payload.extend_from_slice(&client_token);

    write_adb_packet(stream, ADB_PACKET_TYPE_SPAKE2, &initial_payload).await?;

    let (packet_type, server_spake) = read_adb_packet(stream).await?;
    if packet_type != ADB_PACKET_TYPE_SPAKE2 || server_spake.len() < TOKEN_LEN + TAG_LEN {
        return Err(PairingError::AdbPairingFailed(
            "Malformed SPAKE2 response from server".into(),
        ));
    }

    let mut server_token = [0u8; TOKEN_LEN];
    server_token.copy_from_slice(&server_spake[..TOKEN_LEN]);
    let server_tag = &server_spake[TOKEN_LEN..TOKEN_LEN + TAG_LEN];

    let (client_auth, expected_server_auth) = derive_keys(pin, &client_token, &server_token)?;

    if !bool::from(server_tag.ct_eq(&expected_server_auth)) {
        let _ = stream.shutdown().await;
        return Err(PairingError::AdbPinMismatch);
    }

    write_adb_packet(stream, ADB_PACKET_TYPE_SPAKE2, &client_auth).await?;

    let mut info_payload = Vec::with_capacity(4 + client_name.len() + client_key.len());
    info_payload.extend_from_slice(&(client_name.len() as u32).to_be_bytes());
    info_payload.extend_from_slice(client_name.as_bytes());
    info_payload.extend_from_slice(client_key);

    write_adb_packet(stream, ADB_PACKET_TYPE_PEER_INFO, &info_payload).await?;

    let (packet_type, server_info) = read_adb_packet(stream).await?;
    if packet_type != ADB_PACKET_TYPE_PEER_INFO || server_info.len() < 4 {
        return Err(PairingError::AdbPairingFailed(
            "Malformed PeerInfo from server".into(),
        ));
    }

    let name_len = u32::from_be_bytes(server_info[0..4].try_into().unwrap()) as usize;
    if server_info.len() < 4 + name_len {
        return Err(PairingError::AdbPairingFailed(
            "PeerInfo payload truncated".into(),
        ));
    }

    let peer_name = String::from_utf8(server_info[4..4 + name_len].to_vec())
        .map_err(|e| PairingError::AdbPairingFailed(e.to_string()))?;
    let peer_key = server_info[4 + name_len..].to_vec();

    Ok(AdbPeerCredentials {
        name: peer_name,
        key: peer_key,
    })
}

pub async fn pair_server<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    stream: &mut S,
    pin: &str,
    server_name: &str,
    server_key: &[u8],
) -> Result<AdbPeerCredentials, PairingError> {
    if pin.len() != 6 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(PairingError::AdbPairingFailed(
            "PIN must be a 6-digit numeric string".into(),
        ));
    }

    let (packet_type, client_spake) = read_adb_packet(stream).await?;
    if packet_type != ADB_PACKET_TYPE_SPAKE2 || client_spake.len() < TOKEN_LEN {
        return Err(PairingError::AdbPairingFailed(
            "Malformed SPAKE2 packet from client".into(),
        ));
    }

    let mut client_token = [0u8; TOKEN_LEN];
    client_token.copy_from_slice(&client_spake[..TOKEN_LEN]);

    let mut server_token = [0u8; TOKEN_LEN];
    OsRng.fill_bytes(&mut server_token);

    let (expected_client_auth, server_auth) = derive_keys(pin, &client_token, &server_token)?;

    let mut resp = Vec::with_capacity(TOKEN_LEN + TAG_LEN);
    resp.extend_from_slice(&server_token);
    resp.extend_from_slice(&server_auth);

    write_adb_packet(stream, ADB_PACKET_TYPE_SPAKE2, &resp).await?;

    let (packet_type, client_tag) = read_adb_packet(stream).await?;
    if packet_type != ADB_PACKET_TYPE_SPAKE2 || client_tag.len() != TAG_LEN {
        return Err(PairingError::AdbPairingFailed(
            "Malformed client auth tag".into(),
        ));
    }

    if !bool::from(client_tag.as_slice().ct_eq(&expected_client_auth)) {
        let _ = stream.shutdown().await;
        return Err(PairingError::AdbPinMismatch);
    }

    let (packet_type, client_info) = read_adb_packet(stream).await?;
    if packet_type != ADB_PACKET_TYPE_PEER_INFO || client_info.len() < 4 {
        return Err(PairingError::AdbPairingFailed(
            "Malformed PeerInfo from client".into(),
        ));
    }

    let name_len = u32::from_be_bytes(client_info[0..4].try_into().unwrap()) as usize;
    if client_info.len() < 4 + name_len {
        return Err(PairingError::AdbPairingFailed(
            "PeerInfo payload truncated".into(),
        ));
    }

    let peer_name = String::from_utf8(client_info[4..4 + name_len].to_vec())
        .map_err(|e| PairingError::AdbPairingFailed(e.to_string()))?;
    let peer_key = client_info[4 + name_len..].to_vec();

    let mut info_payload = Vec::with_capacity(4 + server_name.len() + server_key.len());
    info_payload.extend_from_slice(&(server_name.len() as u32).to_be_bytes());
    info_payload.extend_from_slice(server_name.as_bytes());
    info_payload.extend_from_slice(server_key);

    write_adb_packet(stream, ADB_PACKET_TYPE_PEER_INFO, &info_payload).await?;

    Ok(AdbPeerCredentials {
        name: peer_name,
        key: peer_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn adb_pairing_roundtrip_success() {
        let (mut client_io, mut server_io) = tokio::io::duplex(4096);
        let pin = "492813";

        let server_task = tokio::spawn(async move {
            pair_server(&mut server_io, pin, "Android Device", b"device_adb_pubkey").await
        });

        let client_creds = pair_client(
            &mut client_io,
            pin,
            "Continue Desktop",
            b"desktop_adb_pubkey",
        )
        .await
        .expect("client pairs");

        let server_creds = server_task.await.unwrap().expect("server pairs");

        assert_eq!(client_creds.name, "Android Device");
        assert_eq!(client_creds.key, b"device_adb_pubkey");

        assert_eq!(server_creds.name, "Continue Desktop");
        assert_eq!(server_creds.key, b"desktop_adb_pubkey");
    }

    #[tokio::test]
    async fn adb_pairing_fails_on_pin_mismatch() {
        let (mut client_io, mut server_io) = tokio::io::duplex(4096);

        let server_task = tokio::spawn(async move {
            pair_server(&mut server_io, "111111", "Android Device", b"device_key").await
        });

        let client_res = pair_client(&mut client_io, "222222", "Desktop", b"desktop_key").await;
        drop(client_io);

        let server_res = server_task.await.unwrap();
        assert!(matches!(client_res, Err(PairingError::AdbPinMismatch)));
        assert!(server_res.is_err());
    }

    #[tokio::test]
    async fn adb_pairing_rejects_invalid_pin_format() {
        let (mut client_io, _server_io) = tokio::io::duplex(4096);
        let res = pair_client(&mut client_io, "12ab", "Desktop", b"key").await;
        assert!(matches!(res, Err(PairingError::AdbPairingFailed(_))));
    }
}
