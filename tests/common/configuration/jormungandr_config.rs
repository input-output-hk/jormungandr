#![allow(dead_code)]

use common::configuration::genesis_model::GenesisYaml;
use common::configuration::node_config_model::NodeConfig;
use std::path::PathBuf;

#[derive(Debug)]
pub struct JormungandrConfig {
    pub genesis_block_path: PathBuf,
    pub genesis_block_hash: String,
    pub genesis_yaml: GenesisYaml,
    pub node_config: NodeConfig,
}

impl JormungandrConfig {
    pub fn get_node_address(&self) -> String {
        self.node_config.get_node_address()
    }

    pub fn new() -> JormungandrConfig {
        JormungandrConfig {
            genesis_block_path: PathBuf::from(""),
            genesis_block_hash: String::from(""),
            genesis_yaml: GenesisYaml::new(),
            node_config: NodeConfig::new(),
        }
    }

    pub fn from(genesis_yaml: GenesisYaml, node_config: NodeConfig) -> JormungandrConfig {
        JormungandrConfig {
            genesis_block_path: PathBuf::from(""),
            genesis_block_hash: String::from(""),
            genesis_yaml: genesis_yaml,
            node_config: node_config,
        }
    }
}
