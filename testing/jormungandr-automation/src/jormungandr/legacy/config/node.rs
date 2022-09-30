use super::NodeConfig;
use crate::jormungandr::configuration::TestConfig;
use jormungandr_lib::multiaddr::to_tcp_socket_addr;
use multiaddr::Multiaddr;
use std::{net::SocketAddr, path::Path};

impl TestConfig for NodeConfig {
    fn log_file_path(&self) -> Option<&Path> {
        self.log.as_ref().and_then(|log| log.file_path())
    }

    fn p2p_listen_address(&self) -> SocketAddr {
        if let Some(address) = &self.p2p.listen {
            *address
        } else {
            to_tcp_socket_addr(&self.p2p.public_address).unwrap()
        }
    }

    fn p2p_public_address(&self) -> Multiaddr {
        self.p2p.public_address.clone()
    }

    fn set_p2p_public_address(&mut self, address: Multiaddr) {
        self.p2p.public_address = address;
    }

    fn rest_socket_addr(&self) -> SocketAddr {
        self.rest.listen
    }

    fn set_rest_socket_addr(&mut self, addr: SocketAddr) {
        self.rest.listen = addr;
    }
}
