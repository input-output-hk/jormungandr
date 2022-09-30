mod blockchain;
mod committee;
mod spawn_params;
mod vote_plan;
mod wallet;

pub use crate::config::{
    blockchain::{Blockchain, BlockchainBuilder},
    committee::CommitteeTemplate,
    spawn_params::SpawnParams,
    wallet::{WalletTemplate, WalletTemplateBuilder},
};
use crate::{
    builder::{Node, NodeAlias, Topology},
    error::Error,
};
use jormungandr_automation::jormungandr::{
    explorer::configuration::ExplorerParams, LogLevel, PersistenceMode, TestingDirectory,
};
use serde::Deserialize;
use std::{collections::HashSet, path::PathBuf, str::FromStr};
pub use vote_plan::VotePlanTemplate;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub blockchain: Blockchain,
    pub nodes: Vec<NodeConfig>,
    pub explorer: Option<ExplorerTemplate>,
    #[serde(default)]
    pub session: SessionSettings,
    pub wallets: Vec<WalletTemplate>,
    pub committees: Vec<CommitteeTemplate>,
    pub vote_plans: Vec<VotePlanTemplate>,
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

    pub fn build_blockchain(&self) -> Blockchain {
        let mut blockchain = self.blockchain.clone();
        for node_config in &self.nodes {
            if node_config.spawn_params.is_leader() {
                blockchain = blockchain.with_leader(node_config.spawn_params.get_alias());
            }
        }
        blockchain
    }

    pub fn node_spawn_params(&self, alias: &str) -> Result<SpawnParams, Error> {
        Ok(self
            .nodes
            .iter()
            .find(|c| c.spawn_params.get_alias() == alias)
            .map(|c| &c.spawn_params)
            .ok_or_else(|| Error::Internal(format!("Node '{}' has no spawn parameters", alias)))?
            .clone()
            .jormungandr(self.session.jormungandr.to_path_buf())
            .log_level(self.session.log.clone()))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct NodeConfig {
    pub spawn_params: SpawnParams,
    #[serde(default)]
    pub trusted_peers: HashSet<NodeAlias>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SessionSettings {
    #[serde(default = "default_jormungandr")]
    pub jormungandr: PathBuf,
    #[serde(default = "default_root")]
    pub root: TestingDirectory,
    #[serde(default)]
    pub generate_documentation: bool,
    pub mode: SessionMode,
    #[serde(default = "default_log_level")]
    pub log: LogLevel,
    #[serde(default = "default_title")]
    pub title: String,
}

fn default_jormungandr() -> PathBuf {
    PathBuf::from_str("jormungandr").unwrap()
}

fn default_log_level() -> LogLevel {
    LogLevel::INFO
}

fn default_root() -> TestingDirectory {
    TestingDirectory::new_temp().unwrap()
}

fn default_title() -> String {
    "unnamed_scenario".to_owned()
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExplorerTemplate {
    pub connect_to: NodeAlias,
    #[serde(default = "default_persistence_mode")]
    pub persistence_mode: PersistenceMode,
    pub address_bech32_prefix: Option<String>,
    pub query_depth_limit: Option<u64>,
    pub query_complexity_limit: Option<u64>,
}

fn default_persistence_mode() -> PersistenceMode {
    PersistenceMode::InMemory
}

impl ExplorerTemplate {
    pub fn to_explorer_params(&self) -> ExplorerParams {
        ExplorerParams {
            address_bech32_prefix: self.address_bech32_prefix.clone(),
            query_complexity_limit: self.query_complexity_limit,
            query_depth_limit: self.query_depth_limit,
        }
    }
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            jormungandr: default_jormungandr(),
            root: default_root(),
            mode: SessionMode::Standard,
            log: default_log_level(),
            generate_documentation: false,
            title: default_title(),
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    Monitor,
    Standard,
    Interactive,
}

pub fn parse_session_mode_from_str(session_mode: &str) -> SessionMode {
    let session_mode_lowercase: &str = &session_mode.to_lowercase();
    match session_mode_lowercase {
        "interactive" => SessionMode::Interactive,
        "monitor" => SessionMode::Monitor,
        _ => SessionMode::Standard,
    }
}
