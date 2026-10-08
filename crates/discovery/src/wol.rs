// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::net::{SocketAddr, UdpSocket};
use thiserror::Error;

pub const WOL_PACKET_LEN: usize = 102;
pub const DEFAULT_WOL_PORT: u16 = 9;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WolError {
    #[error("Invalid MAC address length: expected 6 bytes, found {0}")]
    InvalidLength(usize),

    #[error("Invalid MAC address string: {0}")]
    InvalidFormat(String),

    #[error("Network I/O error: {0}")]
    Io(String),
}

/// Parses a MAC address from a string using either colon or hyphen delimiters (e.g. "AA:BB:CC:DD:EE:FF" or "aa-bb-cc-dd-ee-ff").
pub fn parse_mac_address(mac_str: &str) -> Result<[u8; 6], WolError> {
    let clean = mac_str.trim();
    let parts: Vec<&str> = if clean.contains(':') {
        clean.split(':').collect()
    } else if clean.contains('-') {
        clean.split('-').collect()
    } else {
        return Err(WolError::InvalidFormat(mac_str.to_string()));
    };

    if parts.len() != 6 {
        return Err(WolError::InvalidLength(parts.len()));
    }

    let mut mac = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        if part.len() != 2 {
            return Err(WolError::InvalidFormat(mac_str.to_string()));
        }
        mac[i] = u8::from_str_radix(part, 16)
            .map_err(|_| WolError::InvalidFormat(mac_str.to_string()))?;
    }

    Ok(mac)
}

/// Constructs a 102-byte Wake-on-LAN magic packet consisting of 6 synchronization bytes (0xFF)
/// followed by 16 repetitions of the target device's MAC address.
pub fn create_magic_packet(mac: &[u8; 6]) -> [u8; WOL_PACKET_LEN] {
    let mut packet = [0u8; WOL_PACKET_LEN];
    packet[..6].fill(0xFF);

    for i in 0..16 {
        let start = 6 + i * 6;
        packet[start..start + 6].copy_from_slice(mac);
    }

    packet
}

/// Broadcasts a Wake-on-LAN magic packet across the local network to wake a sleeping host.
pub fn send_wake_on_lan(mac: &[u8; 6], broadcast_addr: Option<SocketAddr>) -> Result<(), WolError> {
    let target_addr: SocketAddr = broadcast_addr.unwrap_or_else(|| {
        "255.255.255.255:9"
            .parse()
            .expect("Valid default broadcast address")
    });

    let socket = UdpSocket::bind("0.0.0.0:0")
        .map_err(|e| WolError::Io(format!("Failed to bind UDP socket: {e}")))?;

    socket
        .set_broadcast(true)
        .map_err(|e| WolError::Io(format!("Failed to enable broadcast on UDP socket: {e}")))?;

    let packet = create_magic_packet(mac);

    socket
        .send_to(&packet, target_addr)
        .map_err(|e| WolError::Io(format!("Failed to broadcast magic packet: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mac_address_colon_and_hyphen() {
        let colon = "AA:BB:CC:11:22:33";
        let parsed_colon = parse_mac_address(colon).unwrap();
        assert_eq!(parsed_colon, [0xAA, 0xBB, 0xCC, 0x11, 0x22, 0x33]);

        let hyphen = "aa-bb-cc-11-22-33";
        let parsed_hyphen = parse_mac_address(hyphen).unwrap();
        assert_eq!(parsed_hyphen, [0xAA, 0xBB, 0xCC, 0x11, 0x22, 0x33]);
    }

    #[test]
    fn parse_mac_address_rejects_invalid() {
        assert!(matches!(
            parse_mac_address("AA:BB:CC:DD"),
            Err(WolError::InvalidLength(4))
        ));
        assert!(matches!(
            parse_mac_address("not_a_mac_address"),
            Err(WolError::InvalidFormat(_))
        ));
        assert!(matches!(
            parse_mac_address("AA:BB:CC:DD:EE:GG"),
            Err(WolError::InvalidFormat(_))
        ));
    }

    #[test]
    fn magic_packet_structure() {
        let mac = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];
        let packet = create_magic_packet(&mac);

        assert_eq!(packet.len(), 102);
        assert_eq!(&packet[..6], &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);

        for i in 0..16 {
            let offset = 6 + i * 6;
            assert_eq!(&packet[offset..offset + 6], &mac);
        }
    }

    #[test]
    fn send_wake_on_lan_loopback() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver_addr = receiver.local_addr().unwrap();

        let mac = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02];
        send_wake_on_lan(&mac, Some(receiver_addr)).unwrap();

        let mut buf = [0u8; 256];
        let (len, _) = receiver.recv_from(&mut buf).unwrap();

        assert_eq!(len, 102);
        assert_eq!(&buf[..102], &create_magic_packet(&mac));
    }
}
