use multiaddr::Multiaddr;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct CommunicationParams {
    pub(crate) p2p_public_address: Multiaddr,
    pub(crate) p2p_listen_address: SocketAddr,
    pub(crate) rest_socket_addr: SocketAddr,
}
