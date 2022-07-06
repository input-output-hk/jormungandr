#![allow(dead_code)]

use super::TestConfig;
use chain_impl_mockchain::{block::Block, fee::LinearFee, fragment::Fragment};
use jormungandr_lib::interfaces::{Address, Block0Configuration, NodeConfig, UTxOInfo};
use serde::Serialize;
use std::{
    fs::File,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct JormungandrParams<Conf = NodeConfig> {
    node_config: Conf,
    node_config_path: PathBuf,
    genesis_block_path: PathBuf,
    genesis_block_hash: String,
    secret_model_path: PathBuf,
    block0_configuration: Block0Configuration,
    rewards_history: bool,
}

impl<Conf: TestConfig> JormungandrParams<Conf> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node_config: Conf,
        node_config_path: impl Into<PathBuf>,
        genesis_block_path: impl Into<PathBuf>,
        genesis_block_hash: impl Into<String>,
        secret_model_path: impl Into<PathBuf>,
        block0_configuration: Block0Configuration,
        rewards_history: bool,
    ) -> Self {
        JormungandrParams {
            node_config,
            node_config_path: node_config_path.into(),
            genesis_block_path: genesis_block_path.into(),
            genesis_block_hash: genesis_block_hash.into(),
            secret_model_path: secret_model_path.into(),
            block0_configuration,
            rewards_history,
        }
    }

    pub fn block0_configuration(&self) -> &Block0Configuration {
        &self.block0_configuration
    }

    pub fn block0_configuration_mut(&mut self) -> &mut Block0Configuration {
        &mut self.block0_configuration
    }

    pub fn genesis_block_path(&self) -> &Path {
        &self.genesis_block_path
    }

    pub fn genesis_block_hash(&self) -> &str {
        &self.genesis_block_hash
    }

    pub fn node_config_path(&self) -> &Path {
        &self.node_config_path
    }

    pub fn rewards_history(&self) -> bool {
        self.rewards_history
    }

    pub fn secret_model_path(&self) -> &Path {
        self.secret_model_path.as_path()
    }

    pub fn rest_uri(&self) -> String {
        format!("http://{}/api", self.node_config.rest_socket_addr())
    }

    pub fn node_config(&self) -> &Conf {
        &self.node_config
    }

    pub fn node_config_mut(&mut self) -> &mut Conf {
        &mut self.node_config
    }

    fn regenerate_ports(&mut self) {
        self.node_config.set_rest_socket_addr(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            super::get_available_port(),
        ));
        self.node_config.set_p2p_public_address(
            format!("/ip4/127.0.0.1/tcp/{}", super::get_available_port())
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

    pub fn fees(&self) -> LinearFee {
        self.block0_configuration
            .blockchain_configuration
            .linear_fees
            .clone()
    }

    pub fn epoch_duration(&self) -> std::time::Duration {
        let slot_duration: u8 = self
            .block0_configuration
            .blockchain_configuration
            .slot_duration
            .into();

        let slots_per_epoch: u32 = self
            .block0_configuration
            .blockchain_configuration
            .slots_per_epoch
            .into();
        std::time::Duration::from_secs(slot_duration as u64 * slots_per_epoch as u64)
    }

    pub fn get_p2p_listen_port(&self) -> u16 {
        self.node_config.p2p_listen_address().port()
    }

    pub fn block0_utxo(&self) -> Vec<UTxOInfo> {
        let block0_bytes = std::fs::read(self.genesis_block_path()).unwrap_or_else(|_| {
            panic!(
                "Failed to load block 0 binary file '{}'",
                self.genesis_block_path().display()
            )
        });

        <Block as chain_core::property::DeserializeFromSlice>::deserialize_from_slice(
            &mut chain_core::packer::Codec::new(block0_bytes.as_slice()),
        )
        .unwrap_or_else(|_| {
            panic!(
                "Failed to parse block in block 0 file '{}'",
                self.genesis_block_path().display()
            )
        })
        .contents()
        .iter()
        .filter_map(|fragment| match fragment {
            Fragment::Transaction(transaction) => Some((transaction, fragment.hash())),
            _ => None,
        })
        .flat_map(|(transaction, fragment_id)| {
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
        .collect()
    }

    pub fn block0_utxo_for_address(&self, address: &Address) -> UTxOInfo {
        let utxo = self
            .block0_utxo()
            .into_iter()
            .find(|utxo| utxo.address() == address)
            .unwrap_or_else(|| panic!("No UTxO found in block 0 for address '{:?}'", address));
        println!("Utxo found for address {}: {:?}", address, &utxo);
        utxo
    }
}

impl<Conf: TestConfig + Serialize> JormungandrParams<Conf> {
    pub fn write_node_config(&self) {
        let mut output_file = File::create(&self.node_config_path).unwrap();
        serde_yaml::to_writer(&mut output_file, &self.node_config)
            .expect("cannot serialize node config");
    }

    pub fn refresh_instance_params(&mut self) {
        self.regenerate_ports();
        self.write_node_config();
        self.recreate_log_file();
    }
}
