use multiaddr::{AddrComponent, Multiaddr, ToMultiaddr};
use std::{
    io,
    net::{SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Not enough components in multiaddr")]
    InvalidMultiaddr,
    #[error("Cannot find DNS record")]
    NotFound(#[source] io::Error),
    #[error("No IPv4 address found")]
    NoIP4,
    #[error("No IPv6 address found")]
    NoIP6,
}

pub fn multiaddr_to_socket_addr(addr: &Multiaddr) -> Option<SocketAddr> {
    let mut components = addr.iter();

    let ip = components.next()?;
    let port = if let AddrComponent::TCP(port) = components.next()? {
        port
    } else {
        return None;
    };

    match ip {
        AddrComponent::IP4(ipv4) => Some(SocketAddr::V4(SocketAddrV4::new(ipv4, port))),
        AddrComponent::IP6(ipv6) => Some(SocketAddr::V6(SocketAddrV6::new(ipv6, port, 0, 0))),
        _ => None,
    }
}

pub fn multiaddr_resolve_dns(addr: &Multiaddr) -> Result<Option<Multiaddr>, Error> {
    let mut components = addr.iter();

    let ip_or_fqdn = components.next().ok_or(Error::InvalidMultiaddr)?;
    let port = components
        .next()
        .and_then(|ac| {
            if let AddrComponent::TCP(port) = ac {
                Some(port)
            } else {
                None
            }
        })
        .ok_or(Error::InvalidMultiaddr)?;

    let socket_addr = match ip_or_fqdn {
        AddrComponent::DNS4(fqdn) => (fqdn.as_str(), port)
            .to_socket_addrs()
            .map_err(Error::NotFound)?
            .find(|addr| matches!(addr, SocketAddr::V4(_)))
            .ok_or(Error::NoIP4)?,
        AddrComponent::DNS6(fqdn) => (fqdn.as_str(), port)
            .to_socket_addrs()
            .map_err(Error::NotFound)?
            .find(|addr| matches!(addr, SocketAddr::V6(_)))
            .ok_or(Error::NoIP6)?,
        _ => return Ok(None),
    };

    Ok(Some(socket_addr.to_multiaddr().unwrap()))
}
