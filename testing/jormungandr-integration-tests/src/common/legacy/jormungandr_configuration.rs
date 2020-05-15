#![allow(dead_code)]

use crate::common::configuration::get_available_port;
use crate::common::file_utils;
use chain_impl_mockchain::fee::LinearFee;
use jormungandr_lib::interfaces::{Block0Configuration, NodeSecret};
use jormungandr_testing_utils::legacy::NodeConfig;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct BackwardCompatibleConfig {
    pub genesis_block_path: PathBuf,
    pub genesis_block_hash: String,
    pub node_config_path: PathBuf,
    pub secret_model_paths: Vec<PathBuf>,
    pub block0_configuration: Block0Configuration,
    pub secret_models: Vec<NodeSecret>,
    pub log_file_path: PathBuf,
    pub rewards_history: bool,
}

impl BackwardCompatibleConfig {
    pub fn new(
        genesis_block_path: PathBuf,
        genesis_block_hash: String,
        node_config_path: PathBuf,
        secret_model_paths: Vec<PathBuf>,
        log_file_path: PathBuf,
        block0_configuration: Block0Configuration,
        secret_models: Vec<NodeSecret>,
        rewards_history: bool,
    ) -> Self {
        Self {
            genesis_block_path,
            genesis_block_hash,
            node_config_path,
            secret_model_paths,
            log_file_path,
            block0_configuration,
            secret_models,
            rewards_history,
        }
    }

    pub fn get_node_address(&self) -> String {
        format!("http://{}/api", self.deserialize_node_config().rest.listen)
    }

    pub fn refresh_node_dynamic_params(&mut self) {
        self.regenerate_ports();
        self.log_file_path = file_utils::get_path_in_temp("log_file.log");
    }

    fn deserialize_node_config(&self) -> NodeConfig {
        serde_yaml::from_str(&file_utils::read_file(&self.node_config_path))
            .expect("cannot deserialize legacy")
    }

    fn serialize_node_config(&mut self, model: NodeConfig) {
        let content = serde_yaml::to_string(&model).expect("cannot serialize legacy");
        self.node_config_path = file_utils::create_file_in_temp("node_config.xml", &content);
    }

    fn regenerate_ports(&mut self) {
        let mut node_config = self.deserialize_node_config();
        node_config.rest.listen = format!("127.0.0.1:{}", get_available_port().to_string())
            .parse()
            .unwrap();
        node_config.p2p.public_address =
            format!("/ip4/127.0.0.1/tcp/{}", get_available_port().to_string())
                .parse()
                .unwrap();
        self.serialize_node_config(node_config);
    }

    pub fn get_p2p_listen_port(&self) -> u16 {
        let address = self
            .deserialize_node_config()
            .p2p
            .public_address
            .to_string();
        let tokens: Vec<&str> = address.split("/").collect();
        let port_str = tokens
            .get(4)
            .expect("cannot extract port from p2p.public_address");
        port_str.parse().unwrap()
    }

    pub fn fees(&self) -> LinearFee {
        self.block0_configuration
            .blockchain_configuration
            .linear_fees
    }
}
