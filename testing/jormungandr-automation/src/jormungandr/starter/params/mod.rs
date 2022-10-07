mod builder;
mod communication;

use crate::jormungandr::{get_available_port, starter::NodeBlock0, LeadershipMode};
pub use builder::JormungandrBootstrapper;
pub use communication::CommunicationParams;
use multiaddr::Multiaddr;
use std::{
    fmt::Debug,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

/// Abstraction over different versions of the node configuration.
pub trait ConfigurableNodeConfig: Debug {
    fn log_file_path(&self) -> Option<&Path>;
    fn write_node_config(&self);
    fn node_config_path(&self) -> PathBuf;
    fn set_node_config_path(&mut self, path: PathBuf);
    fn p2p_listen_address(&self) -> SocketAddr;
    fn p2p_public_address(&self) -> Multiaddr;
    fn set_p2p_public_address(&mut self, address: Multiaddr);
    fn rest_socket_addr(&self) -> SocketAddr;
    fn set_rest_socket_addr(&mut self, addr: SocketAddr);
    fn as_communication_params(&self) -> CommunicationParams;
}

#[derive(Debug)]
pub struct JormungandrParams {
    node_config: Box<dyn ConfigurableNodeConfig>,
    genesis: NodeBlock0,
    secret_path: Option<PathBuf>,
    leadership: LeadershipMode,
    rewards_history: bool,
}

impl JormungandrParams {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node_config: Box<dyn ConfigurableNodeConfig>,
        genesis: NodeBlock0,
        secret_path: Option<PathBuf>,
        leadership: LeadershipMode,
        rewards_history: bool,
    ) -> Self {
        JormungandrParams {
            node_config,
            genesis,
            secret_path,
            leadership,
            rewards_history,
        }
    }

    pub(crate) fn comm(&self) -> CommunicationParams {
        self.node_config.as_communication_params()
    }

    pub(crate) fn genesis(&self) -> &NodeBlock0 {
        &self.genesis
    }

    pub fn node_config_path(&self) -> PathBuf {
        self.node_config.node_config_path()
    }

    pub fn refresh_instance_params(&mut self) {
        self.regenerate_ports();
        self.node_config.write_node_config();
        self.recreate_log_file();
    }

    pub fn get_p2p_listen_port(&self) -> u16 {
        self.node_config.p2p_listen_address().port()
    }

    fn regenerate_ports(&mut self) {
        self.node_config.set_rest_socket_addr(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            get_available_port(),
        ));
        self.node_config.set_p2p_public_address(
            format!("/ip4/127.0.0.1/tcp/{}", get_available_port())
                .parse()
                .unwrap(),
        );
    }

    fn recreate_log_file(&mut self) {
        if let Some(path) = self.node_config.log_file_path() {
            std::fs::remove_file(path).unwrap_or_else(|e| {
                println!(
                    "Failed to remove log file {}: {}",
                    path.to_string_lossy(),
                    e
                );
            });
        }
    }
    pub fn secret_path(&self) -> &Option<PathBuf> {
        &self.secret_path
    }

    pub fn leadership(&self) -> LeadershipMode {
        self.leadership
    }

    pub fn rewards_history(&self) -> bool {
        self.rewards_history
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Serde(#[from] serde_yaml::Error),
    #[error(transparent)]
    Fixture(#[from] assert_fs::fixture::FixtureError),
    #[error(transparent)]
    Write(#[from] chain_core::property::WriteError),
    #[error("block0 source not defined")]
    Block0SourceNotDefined,
    #[error("apply minimal setup failed: cannot modify block0 as only block hash is provided")]
    CannotApplyMinimalSetupDueToHash,
}
