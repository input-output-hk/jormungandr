use multiaddr::{AddrComponent, Multiaddr, ToMultiaddr};
use std::{
    io,
    net::{SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs},
};

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

pub fn multiaddr_resolve_dns(addr: Multiaddr) -> io::Result<Option<Multiaddr>> {
    let mut components = addr.iter();

    let ip_or_fqdn = if let Some(ip) = components.next() {
        ip
    } else {
        return Ok(None);
    };

    let port = if let Some(AddrComponent::TCP(port)) = components.next() {
        port
    } else {
        return Ok(None);
    };

    let maybe_socket_addr = match ip_or_fqdn {
        AddrComponent::DNS4(fqdn) => (fqdn.as_ref(), port)
            .to_socket_addrs()?
            .find(|addr| matches!(addr, SocketAddr::V4(_))),
        AddrComponent::DNS6(fqdn) => (fqdn.as_ref(), port)
            .to_socket_addrs()?
            .find(|addr| matches!(addr, SocketAddr::V6(_))),
        _ => return Ok(Some(addr)),
    };

    Ok(maybe_socket_addr.map(|socket_addr| socket_addr.to_multiaddr().unwrap()))
}
