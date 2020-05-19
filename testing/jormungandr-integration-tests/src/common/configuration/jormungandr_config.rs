#![allow(dead_code)]

use crate::common::{
    configuration::NodeConfigBuilder, file_utils, jormungandr::ConfigurationBuilder,
    legacy::BackwardCompatibleConfig,
};
use chain_core::mempack;
use chain_impl_mockchain::{block::Block, fee::LinearFee, fragment::Fragment};
use jormungandr_lib::interfaces::{
    Block0Configuration, NodeConfig, NodeSecret, TrustedPeer, UTxOInfo,
};
use jormungandr_testing_utils::wallet::Wallet;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct JormungandrConfig {
    inner: BackwardCompatibleConfig,
}

impl JormungandrConfig {
    pub fn new(
        genesis_block_path: PathBuf,
        genesis_block_hash: String,
        node_config_path: PathBuf,
        secret_model_paths: Vec<PathBuf>,
        block0_configuration: Block0Configuration,
        secret_models: Vec<NodeSecret>,
        rewards_history: bool,
    ) -> Self {
        Self {
            inner: BackwardCompatibleConfig {
                genesis_block_path,
                genesis_block_hash,
                node_config_path,
                secret_model_paths,
                block0_configuration,
                secret_models,
                rewards_history,
            },
        }
    }

    pub fn block0_configuration(&self) -> &Block0Configuration {
        &self.inner.block0_configuration
    }

    pub fn block0_configuration_mut(&mut self) -> &mut Block0Configuration {
        &mut self.inner.block0_configuration
    }

    pub fn genesis_block_path(&self) -> &PathBuf {
        &self.inner.genesis_block_path
    }

    pub fn genesis_block_hash(&self) -> &String {
        &self.inner.genesis_block_hash
    }

    pub fn node_config_path(&self) -> &PathBuf {
        &self.inner.node_config_path
    }

    pub fn rewards_history(&self) -> bool {
        self.inner.rewards_history
    }

    pub fn log_file_path(&self) -> Option<PathBuf> {
        self.inner.log_file_path()
    }

    pub fn secret_model_paths_mut(&mut self) -> &mut Vec<PathBuf> {
        &mut self.inner.secret_model_paths
    }

    pub fn secret_models_mut(&mut self) -> &mut Vec<NodeSecret> {
        &mut self.inner.secret_models
    }

    pub fn secret_model_paths(&self) -> &Vec<PathBuf> {
        &self.inner.secret_model_paths
    }

    pub fn secret_models(&self) -> &Vec<NodeSecret> {
        &self.inner.secret_models
    }

    pub fn get_node_address(&self) -> String {
        format!("http://{}/api", self.node_config().rest.listen)
    }

    pub fn node_config(&self) -> NodeConfig {
        let content = file_utils::read_file(&self.inner.node_config_path);
        serde_yaml::from_str(&content).expect("Canot serialize node config")
    }

    pub fn refresh_node_dynamic_params(&mut self) {
        let node_config = self.regenerate_ports();
        self.update_node_config(node_config);
    }

    fn update_node_config(&mut self, node_config: NodeConfig) {
        self.inner.node_config_path = NodeConfigBuilder::serialize(&node_config);
    }

    fn regenerate_ports(&mut self) -> NodeConfig {
        let mut node_config = self.node_config();
        node_config.rest.listen = format!("127.0.0.1:{}", super::get_available_port().to_string())
            .parse()
            .unwrap();
        node_config.p2p.public_address = format!(
            "/ip4/127.0.0.1/tcp/{}",
            super::get_available_port().to_string()
        )
        .parse()
        .unwrap();
        node_config
    }

    pub fn fees(&self) -> LinearFee {
        self.inner
            .block0_configuration
            .blockchain_configuration
            .linear_fees
            .clone()
    }

    pub fn get_p2p_listen_port(&self) -> u16 {
        let address = self.node_config().p2p.get_listen_address().to_string();
        let tokens: Vec<&str> = address.split("/").collect();
        let port_str = tokens
            .get(4)
            .expect("cannot extract port from p2p.public_address");
        port_str.parse().unwrap()
    }

    pub fn as_trusted_peer(&self) -> TrustedPeer {
        self.node_config().p2p.make_trusted_peer_setting()
    }

    pub fn block0_utxo(&self) -> Vec<UTxOInfo> {
        let block0_bytes = std::fs::read(self.genesis_block_path()).expect(&format!(
            "Failed to load block 0 binary file '{}'",
            self.genesis_block_path().display()
        ));
        mempack::read_from_raw::<Block>(&block0_bytes)
            .expect(&format!(
                "Failed to parse block in block 0 file '{}'",
                self.genesis_block_path().display()
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

impl Default for JormungandrConfig {
    fn default() -> JormungandrConfig {
        ConfigurationBuilder::new().build()
    }
}

impl Into<BackwardCompatibleConfig> for JormungandrConfig {
    fn into(self) -> BackwardCompatibleConfig {
        self.inner
    }
}
