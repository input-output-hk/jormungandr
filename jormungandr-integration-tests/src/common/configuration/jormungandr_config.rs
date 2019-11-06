#![allow(dead_code)]

use crate::common::configuration::genesis_model::GenesisYaml;
use crate::common::configuration::node_config_model::NodeConfig;
use crate::common::configuration::secret_model::SecretModel;
use crate::common::data::address::AddressDataProvider;
use crate::common::file_utils;
use chain_core::mempack;
use chain_impl_mockchain::block::Block;
use chain_impl_mockchain::fragment::Fragment;
use jormungandr_lib::interfaces::UTxOInfo;
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

    pub fn refresh_node_dynamic_params(&mut self) {
        self.node_config.regenerate_ports();
        self.update_node_config();
        self.log_file_path = file_utils::get_path_in_temp("log_file.log");
    }

    pub fn update_node_config(&mut self) {
        self.node_config_path = NodeConfig::serialize(&self.node_config);
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

    pub fn block0_utxo_for_address<T: AddressDataProvider>(&self, address: &T) -> UTxOInfo {
        let utxo_address = address.get_address();
        self.block0_utxo()
            .into_iter()
            .find(|utxo| utxo.address().to_string() == utxo_address)
            .expect(&format!(
                "No UTxO found in block 0 for address '{}' of type '{}'",
                utxo_address,
                address.get_address_type()
            ))
    }
}
