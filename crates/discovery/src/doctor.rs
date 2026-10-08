// SPDX-FileCopyrightText: Contributors to the Continue project
// SPDX-License-Identifier: Apache-2.0

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceKind {
    Loopback,
    PrivateLan,
    LinkLocal,
    VirtualOrVpn,
    Public,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceReport {
    pub ip: IpAddr,
    pub kind: InterfaceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkIssue {
    NoActiveLanInterface,
    PortUnavailable { port: u16 },
    SubnetMismatch { peer_ip: IpAddr },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDiagnostic {
    pub interfaces: Vec<InterfaceReport>,
    pub issues: Vec<NetworkIssue>,
}

pub fn classify_ip(ip: IpAddr) -> InterfaceKind {
    match ip {
        IpAddr::V4(v4) => {
            if v4.is_loopback() {
                InterfaceKind::Loopback
            } else if v4.is_link_local() {
                InterfaceKind::LinkLocal
            } else if is_common_virtual_subnet(v4) {
                // WSL2, Hyper-V, and Docker routinely allocate 172.17-172.31 ranges,
                // which isolate virtual bridges from the host Wi-Fi adapter.
                InterfaceKind::VirtualOrVpn
            } else if v4.is_private() {
                InterfaceKind::PrivateLan
            } else {
                InterfaceKind::Public
            }
        }
        IpAddr::V6(v6) => {
            if v6.is_loopback() {
                InterfaceKind::Loopback
            } else if (v6.segments()[0] & 0xffc0) == 0xfe80 {
                InterfaceKind::LinkLocal
            } else {
                InterfaceKind::PrivateLan
            }
        }
    }
}

fn is_common_virtual_subnet(v4: Ipv4Addr) -> bool {
    let octets = v4.octets();
    // Docker default bridge: 172.17.0.0/16
    // WSL2 / Hyper-V vEthernet: dynamic assignments within 172.18.0.0/16 - 172.31.0.0/16
    octets[0] == 172 && (17..=31).contains(&octets[1])
}

fn is_same_class_c_subnet(a: Ipv4Addr, b: Ipv4Addr) -> bool {
    let o1 = a.octets();
    let o2 = b.octets();
    o1[0] == o2[0] && o1[1] == o2[1] && o1[2] == o2[2]
}

pub fn probe_port_bindable(port: u16) -> bool {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    UdpSocket::bind(addr).is_ok()
}

pub fn diagnose_network(
    detected_ips: &[IpAddr],
    peer_ip: Option<IpAddr>,
    listen_port: u16,
) -> NetworkDiagnostic {
    let mut interfaces = Vec::with_capacity(detected_ips.len());
    let mut issues = Vec::new();

    let mut has_lan = false;

    for &ip in detected_ips {
        let kind = classify_ip(ip);
        if kind == InterfaceKind::PrivateLan {
            has_lan = true;
        }
        interfaces.push(InterfaceReport { ip, kind });
    }

    if !has_lan {
        issues.push(NetworkIssue::NoActiveLanInterface);
    }

    if !probe_port_bindable(listen_port) {
        issues.push(NetworkIssue::PortUnavailable { port: listen_port });
    }

    if let (Some(IpAddr::V4(peer_v4)), true) = (peer_ip, has_lan) {
        let matches_any = interfaces.iter().any(|iface| match iface.ip {
            IpAddr::V4(local_v4) if iface.kind == InterfaceKind::PrivateLan => {
                is_same_class_c_subnet(local_v4, peer_v4)
            }
            _ => false,
        });

        if !matches_any {
            issues.push(NetworkIssue::SubnetMismatch {
                peer_ip: IpAddr::V4(peer_v4),
            });
        }
    }

    NetworkDiagnostic { interfaces, issues }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_interface_kinds() {
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            InterfaceKind::Loopback
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 2))),
            InterfaceKind::LinkLocal
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50))),
            InterfaceKind::PrivateLan
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 15))),
            InterfaceKind::PrivateLan
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(172, 28, 0, 1))),
            InterfaceKind::VirtualOrVpn
        );
        assert_eq!(
            classify_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))),
            InterfaceKind::Public
        );
    }

    #[test]
    fn detects_missing_lan_interface() {
        let ips = [
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(172, 28, 0, 1)),
        ];
        let report = diagnose_network(&ips, None, 0);
        assert!(report.issues.contains(&NetworkIssue::NoActiveLanInterface));
    }

    #[test]
    fn detects_subnet_mismatch() {
        let ips = [IpAddr::V4(Ipv4Addr::new(192, 168, 2, 100))];
        let peer = Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)));
        let report = diagnose_network(&ips, peer, 0);

        assert_eq!(
            report.issues,
            vec![NetworkIssue::SubnetMismatch {
                peer_ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50))
            }]
        );
    }

    #[test]
    fn passes_healthy_network() {
        let ips = [IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10))];
        let peer = Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 50)));
        let report = diagnose_network(&ips, peer, 0);
        assert!(report.issues.is_empty());
    }
}
