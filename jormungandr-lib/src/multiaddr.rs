use multiaddr::{AddrComponent, Multiaddr};
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};

pub fn multiaddr_to_socket_addr(addr: &Multiaddr) -> Option<SocketAddr> {
    let mut components = addr.iter();

    match components.next()? {
        AddrComponent::IP4(ipv4) => {
            if let AddrComponent::TCP(port) = components.next()? {
                Some(SocketAddr::V4(SocketAddrV4::new(ipv4, port)))
            } else {
                None
            }
        }
        AddrComponent::IP6(ipv6) => {
            if let AddrComponent::TCP(port) = components.next()? {
                Some(SocketAddr::V6(SocketAddrV6::new(ipv6, port, 0, 0)))
            } else {
                None
            }
        }
        _ => None,
    }
}
