#![allow(dead_code)]

use crate::common::configuration::{Block0ConfigurationBuilder, NodeConfigBuilder};
use crate::common::file_utils;
use chain_core::mempack;
use chain_impl_mockchain::{block::Block, fee::LinearFee, fragment::Fragment};
use jormungandr_lib::{
    interfaces::{Block0Configuration, NodeConfig, NodeSecret, TrustedPeer, UTxOInfo},
    wallet::Wallet,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct JormungandrConfig {
    pub genesis_block_path: PathBuf,
    pub genesis_block_hash: String,
    pub node_config_path: PathBuf,
    pub secret_model_paths: Vec<PathBuf>,
    pub block0_configuration: Block0Configuration,
    pub node_config: NodeConfig,
    pub secret_models: Vec<NodeSecret>,
    pub log_file_path: PathBuf,
    pub rewards_history: bool,
}

impl JormungandrConfig {
    pub fn get_node_address(&self) -> String {
        let rest = &self.node_config.rest;
        let output = format!("http://{}/api", rest.listen);
        output
    }

    pub fn refresh_node_dynamic_params(&mut self) {
        self.regenerate_ports();
        self.update_node_config();
        self.log_file_path = file_utils::get_path_in_temp("log_file.log");
    }

    pub fn update_node_config(&mut self) {
        self.node_config_path = NodeConfigBuilder::serialize(&self.node_config);
    }

    fn regenerate_ports(&mut self) {
        self.node_config.rest.listen =
            format!("127.0.0.1:{}", super::get_available_port().to_string())
                .parse()
                .unwrap();
        self.node_config.p2p.public_address = format!(
            "/ip4/127.0.0.1/tcp/{}",
            super::get_available_port().to_string()
        )
        .parse()
        .unwrap();
    }

    pub fn fees(&self) -> LinearFee {
        self.block0_configuration
            .blockchain_configuration
            .linear_fees
            .clone()
    }

    pub fn get_p2p_listen_port(&self) -> u16 {
        let address = self.node_config.p2p.get_listen_address().to_string();
        let tokens: Vec<&str> = address.split("/").collect();
        let port_str = tokens
            .get(4)
            .expect("cannot extract port from p2p.public_address");
        port_str.parse().unwrap()
    }

    pub fn new() -> Self {
        JormungandrConfig::from(
            Block0ConfigurationBuilder::new().build(),
            NodeConfigBuilder::new().build(),
        )
    }

    pub fn as_trusted_peer(&self) -> TrustedPeer {
        self.node_config.p2p.make_trusted_peer_setting()
    }

    pub fn from(block0_configuration: Block0Configuration, node_config: NodeConfig) -> Self {
        JormungandrConfig {
            genesis_block_path: PathBuf::from(""),
            genesis_block_hash: String::from(""),
            node_config_path: PathBuf::from(""),
            secret_model_paths: Vec::new(),
            log_file_path: PathBuf::from(""),
            block0_configuration: block0_configuration,
            node_config: node_config,
            secret_models: Vec::new(),
            rewards_history: false,
        }
    }

    pub fn block0_utxo(&self) -> Vec<UTxOInfo> {
        let block0_bytes = std::fs::read(&self.genesis_block_path).expect(&format!(
            "Failed to load block 0 binary file '{}'",
            self.genesis_block_path.display()
        ));
        mempack::read_from_raw::<Block>(&block0_bytes)
            .expect(&format!(
                "Failed to parse block in block 0 file '{}'",
                self.genesis_block_path.display()
            ))
            .contents
            .iter()
            .filter_map(|fragment| match fragment {
                Fragment::Transaction(transaction) => Some((transaction, fragment.hash())),
                _ => None,
            })
            .map(|(transaction, fragment_id)| {
                transaction
                    .as_slice()
                    .outputs()
                    .iter()
                    .enumerate()
                    .map(move |(idx, output)| {
                        UTxOInfo::new(
                            fragment_id.into(),
                            idx as u8,
                            output.address.clone().into(),
                            output.value.into(),
                        )
                    })
            })
            .flatten()
            .collect()
    }

    pub fn block0_utxo_for_address(&self, wallet: &Wallet) -> UTxOInfo {
        let utxo = self
            .block0_utxo()
            .into_iter()
            .find(|utxo| *utxo.address() == wallet.address())
            .expect(&format!(
                "No UTxO found in block 0 for address '{:?}'",
                wallet
            ));
        println!(
            "Utxo found for address {}: {:?}",
            wallet.address().to_string(),
            &utxo
        );
        utxo
    }
}
