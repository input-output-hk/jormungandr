use multiaddr::{Multiaddr, Protocol};
use std::{
    borrow::Borrow,
    io,
    net::{IpAddr, SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Multiaddr shall consist of a host address and a TCP component")]
    InvalidMultiaddr,
    #[error("Failed to resolve DNS record")]
    FailedToResolve(#[source] io::Error),
    #[error("No IP addresses found")]
    NotFound,
    #[error("No IPv4 address found")]
    NoIp4,
    #[error("No IPv6 address found")]
    NoIp6,
}

/// Checks if the multiaddr is valid for contacting a p2p peer
/// and resolves DNS components.
///
/// Note that DNS resolution is performed synchronously by this function,
/// so this should only be used at initialization.
pub fn resolve_dns(addr: &Multiaddr) -> Result<Multiaddr, Error> {
    let mut components = addr.iter();

    let ip_or_fqdn = components.next().ok_or(Error::InvalidMultiaddr)?;
    let port = components
        .next()
        .and_then(|ac| {
            if let Protocol::Tcp(port) = ac {
                Some(port)
            } else {
                None
            }
        })
        .ok_or(Error::InvalidMultiaddr)?;

    let socket_addr = match ip_or_fqdn {
        Protocol::Ip4(addr) => SocketAddrV4::new(addr, port).into(),
        Protocol::Ip6(addr) => SocketAddrV6::new(addr, port, 0, 0).into(),
        Protocol::Dns(fqdn) => (fqdn.borrow(), port)
            .to_socket_addrs()
            .map_err(Error::FailedToResolve)?
            .next()
            .ok_or(Error::NotFound)?,
        Protocol::Dns4(fqdn) => (fqdn.borrow(), port)
            .to_socket_addrs()
            .map_err(Error::FailedToResolve)?
            .find(|addr| matches!(addr, SocketAddr::V4(_)))
            .ok_or(Error::NoIp4)?,
        Protocol::Dns6(fqdn) => (fqdn.borrow(), port)
            .to_socket_addrs()
            .map_err(Error::FailedToResolve)?
            .find(|addr| matches!(addr, SocketAddr::V6(_)))
            .ok_or(Error::NoIp6)?,
        _ => return Err(Error::InvalidMultiaddr),
    };

    // Check if the multiaddr has only the expected two components
    // to avoid processing freak cases as valid.
    if components.next().is_some() {
        return Err(Error::InvalidMultiaddr);
    }

    Ok(Multiaddr::from(socket_addr.ip()).with(Protocol::Tcp(socket_addr.port())))
}

/// Extracts the TCP socket address if the multiaddr starts with an
/// `/ip4` or `/ip6` component, followed by a `/tcp` component.
/// Otherwise the function returns `None`.
pub fn to_tcp_socket_addr(addr: &Multiaddr) -> Option<SocketAddr> {
    let mut components = addr.iter();
    let ip = match components.next()? {
        Protocol::Ip4(ip_addr) => IpAddr::V4(ip_addr),
        Protocol::Ip6(ip_addr) => IpAddr::V6(ip_addr),
        _ => return None,
    };
    let port = match components.next()? {
        Protocol::Tcp(port) => port,
        _ => return None,
    };
    Some(SocketAddr::new(ip, port))
}
