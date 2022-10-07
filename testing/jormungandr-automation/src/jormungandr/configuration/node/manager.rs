use crate::jormungandr::starter::{CommunicationParams, ConfigurableNodeConfig};
use jormungandr_lib::{interfaces::NodeConfig, multiaddr::to_tcp_socket_addr};
use multiaddr::Multiaddr;
use std::{
    fmt::{Debug, Formatter},
    fs::File,
    net::SocketAddr,
    path::{Path, PathBuf},
};

pub struct NodeConfigManager {
    pub node_config: NodeConfig,
    pub file: Option<PathBuf>,
}

impl Debug for NodeConfigManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.node_config.fmt(f)
    }
}

impl ConfigurableNodeConfig for NodeConfigManager {
    fn log_file_path(&self) -> Option<&Path> {
        self.node_config
            .log
            .as_ref()
            .and_then(|log| log.file_path())
    }

    fn write_node_config(&self) {
        let mut output_file = File::create(&self.node_config_path()).unwrap();
        serde_yaml::to_writer(&mut output_file, &self.node_config)
            .expect("cannot serialize node config");
    }

    fn node_config_path(&self) -> PathBuf {
        self.file
            .as_ref()
            .expect("node config path not defined")
            .clone()
    }

    fn set_node_config_path(&mut self, path: PathBuf) {
        self.file = Some(path);
    }

    fn p2p_listen_address(&self) -> SocketAddr {
        if let Some(address) = &self.node_config.p2p.listen {
            *address
        } else {
            to_tcp_socket_addr(&self.node_config.p2p.public_address).unwrap()
        }
    }

    fn p2p_public_address(&self) -> Multiaddr {
        self.node_config.p2p.public_address.clone()
    }

    fn set_p2p_public_address(&mut self, address: Multiaddr) {
        self.node_config.p2p.public_address = address;
    }

    fn rest_socket_addr(&self) -> SocketAddr {
        self.node_config.rest.listen
    }

    fn set_rest_socket_addr(&mut self, addr: SocketAddr) {
        self.node_config.rest.listen = addr;
    }

    fn as_communication_params(&self) -> CommunicationParams {
        CommunicationParams {
            p2p_public_address: self.p2p_public_address(),
            p2p_listen_address: self.p2p_listen_address(),
            rest_socket_addr: self.rest_socket_addr(),
        }
    }
}
