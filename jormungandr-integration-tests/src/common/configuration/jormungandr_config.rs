#![allow(dead_code)]

use crate::common::configuration::genesis_model::GenesisYaml;
use crate::common::configuration::node_config_model::NodeConfig;
use crate::common::configuration::secret_model::SecretModel;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct JormungandrConfig {
    pub genesis_block_path: PathBuf,
    pub genesis_block_hash: String,
    pub node_config_path: PathBuf,
    pub secret_model_path: PathBuf,
    pub genesis_yaml: GenesisYaml,
    pub node_config: NodeConfig,
    pub secret_model: SecretModel,
    pub log_file_path: PathBuf,
}

impl JormungandrConfig {
    pub fn get_node_address(&self) -> String {
        self.node_config.get_node_address()
    }

    pub fn new() -> Self {
        JormungandrConfig::from(GenesisYaml::new(), NodeConfig::new())
    }

    pub fn from(genesis_yaml: GenesisYaml, node_config: NodeConfig) -> Self {
        JormungandrConfig {
            genesis_block_path: PathBuf::from(""),
            genesis_block_hash: String::from(""),
            node_config_path: PathBuf::from(""),
            secret_model_path: PathBuf::from(""),
            log_file_path: PathBuf::from(""),
            genesis_yaml: genesis_yaml,
            node_config: node_config,
            secret_model: SecretModel::empty(),
        }
    }
}
