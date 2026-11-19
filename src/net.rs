//! PC IP detection and IPv4 validation (DESIGN 9.6, 4.5).

use std::net::{Ipv4Addr, UdpSocket};

#[derive(Debug, PartialEq, Eq)]
pub enum IpError {
    Empty,
    /// Looks like a host name; RPI 3.58 rejects host names (DNS rebinding protection).
    Hostname,
    Invalid,
    /// 0.0.0.0, 255.255.255.255 or multicast.
    NotUnicast,
}

/// Validates the console IP typed in Settings: IPv4 literal only.
pub fn parse_console_ip(s: &str) -> Result<Ipv4Addr, IpError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(IpError::Empty);
    }
    let ip: Ipv4Addr = s.parse().map_err(|_| {
        if s.chars().any(|c| c.is_ascii_alphabetic()) {
            IpError::Hostname
        } else {
            IpError::Invalid
        }
    })?;
    if ip.is_unspecified() || ip.is_broadcast() || ip.is_multicast() {
        return Err(IpError::NotUnicast);
    }
    Ok(ip)
}

/// IP of the local interface the OS would use to reach the console.
/// UDP `connect` only selects a route; no packet is sent.
pub fn pc_ip_for(console: Ipv4Addr, port: u16) -> std::io::Result<Ipv4Addr> {
    let sock = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    sock.connect((console, port))?;
    match sock.local_addr()?.ip() {
        std::net::IpAddr::V4(ip)
            if !ip.is_unspecified() && (!ip.is_loopback() || console.is_loopback()) =>
        {
            Ok(ip)
        }
        other => Err(std::io::Error::other(format!("no usable route ({other})"))),
    }
}

/// /24 heuristic for the connection test warning (subnet mask is not available from std).
pub fn same_24(a: Ipv4Addr, b: Ipv4Addr) -> bool {
    a.octets()[..3] == b.octets()[..3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_ip_validation() {
        assert_eq!(
            parse_console_ip(" 192.168.1.50 "),
            Ok(Ipv4Addr::new(192, 168, 1, 50))
        );
        assert_eq!(parse_console_ip("127.0.0.1"), Ok(Ipv4Addr::LOCALHOST));
        assert_eq!(parse_console_ip(""), Err(IpError::Empty));
        assert_eq!(parse_console_ip("ps4.local"), Err(IpError::Hostname));
        assert_eq!(parse_console_ip("192.168.1"), Err(IpError::Invalid));
        assert_eq!(parse_console_ip("192.168.1.300"), Err(IpError::Invalid));
        assert_eq!(parse_console_ip("0.0.0.0"), Err(IpError::NotUnicast));
        assert_eq!(
            parse_console_ip("255.255.255.255"),
            Err(IpError::NotUnicast)
        );
        assert_eq!(parse_console_ip("224.0.0.1"), Err(IpError::NotUnicast));
    }

    #[test]
    fn route_to_loopback_console() {
        assert_eq!(
            pc_ip_for(Ipv4Addr::LOCALHOST, 12801).unwrap(),
            Ipv4Addr::LOCALHOST
        );
    }

    #[test]
    fn subnet_heuristic() {
        assert!(same_24(
            Ipv4Addr::new(192, 168, 1, 10),
            Ipv4Addr::new(192, 168, 1, 50)
        ));
        assert!(!same_24(
            Ipv4Addr::new(192, 168, 1, 10),
            Ipv4Addr::new(192, 168, 2, 50)
        ));
    }
}
