use super::NodeConfig;
use crate::testing::node::configuration::TestConfig;
use std::net::SocketAddr;
use std::path::Path;

impl TestConfig for NodeConfig {
    fn log_file_path(&self) -> Option<&Path> {
        self.log.as_ref().and_then(|log| log.file_path())
    }

    fn p2p_listen_address(&self) -> poldercast::Address {
        if let Some(address) = &self.p2p.listen_address {
            address.clone()
        } else {
            self.p2p.public_address.clone()
        }
    }

    fn p2p_public_address(&self) -> poldercast::Address {
        self.p2p.public_address.clone()
    }

    fn set_p2p_public_address(&mut self, address: poldercast::Address) {
        self.p2p.public_address = address;
    }

    fn rest_socket_addr(&self) -> SocketAddr {
        self.rest.listen
    }

    fn set_rest_socket_addr(&mut self, addr: SocketAddr) {
        self.rest.listen = addr;
    }
}
