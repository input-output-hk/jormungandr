use multiaddr::{Multiaddr, Protocol};
use std::borrow::Borrow;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Not enough components in multiaddr")]
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

pub fn multiaddr_resolve_dns(addr: &Multiaddr) -> Result<Option<Multiaddr>, Error> {
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
        _ => return Ok(None),
    };

    let multiaddr = Multiaddr::from(socket_addr.ip()).with(Protocol::Tcp(port));
    Ok(Some(multiaddr))
}
