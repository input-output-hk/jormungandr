use jormungandr_testing_utils::testing::{
    jormungandr::TestingDirectory,
    network::{Blockchain, Node, NodeAlias, SpawnParams, Topology},
    node::LogLevel,
};

use crate::error::Error;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub blockchain: Blockchain,
    pub nodes: Vec<NodeConfig>,
    #[serde(default)]
    pub session: SessionSettings,
}

impl Config {
    pub fn build_topology(&self) -> Topology {
        let mut topology = Topology::default();

        for node_config in self.nodes.iter() {
            let mut node = Node::new(node_config.spawn_params.get_alias());

            for trusted_peer in node_config.trusted_peers.iter() {
                node = node.with_trusted_peer(trusted_peer);
            }

            topology = topology.with_node(node);
        }

        topology
    }

    pub fn node_spawn_params(&self, alias: &str) -> Result<SpawnParams, Error> {
        let mut spawn_params = self
            .nodes
            .iter()
            .find(|c| c.spawn_params.get_alias() == alias)
            .map(|c| &c.spawn_params)
            .ok_or_else(|| Error::Internal(format!("Node '{}' has no spawn parameters", alias)))?
            .clone();

        if let Some(jormungandr) = &self.session.jormungandr {
            spawn_params = spawn_params.jormungandr(jormungandr.to_path_buf());
        }
        Ok(spawn_params.log_level(self.session.log.clone()))
    }

    pub fn testing_directory(&self) -> TestingDirectory {
        match &self.session.root {
            Some(path) => path.to_path_buf().into(),
            None => TestingDirectory::new_temp().unwrap(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NodeConfig {
    pub spawn_params: SpawnParams,
    #[serde(default)]
    pub trusted_peers: HashSet<NodeAlias>,
}

#[derive(Debug, Deserialize)]
pub struct SessionSettings {
    jormungandr: Option<PathBuf>,
    root: Option<PathBuf>,
    #[allow(dead_code)]
    mode: SessionMode,
    #[serde(default)]
    log: LogLevel,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            jormungandr: None,
            root: None,
            mode: SessionMode::Standard,
            log: LogLevel::INFO,
        }
    }
}

//TODO implmement proper reporting by using below settings
#[derive(Debug, Copy, Clone, Deserialize)]
pub enum SessionMode {
    Monitor,
    Standard,
    Interactive,
}
