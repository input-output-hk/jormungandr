#![cfg(feature = "integration-test")]

pub mod jormungandr_starter;

use super::configuration::{
    genesis_model::{Fund, GenesisYaml},
    jormungandr_config::JormungandrConfig,
    node_config_model::NodeConfig,
    secret_model::SecretModel,
};
use super::data::address::{Account, AddressDataProvider, Delegation, Utxo};
use super::data::utxo::Utxo as UtxoData;
use super::file_utils;
use super::jcli_wrapper;

use common::process_utils::process_guard::ProcessKillGuard;
use std::path::PathBuf;

pub fn start_jormungandr_node(mut config: &mut JormungandrConfig) -> ProcessKillGuard {
    jormungandr_starter::start_jormungandr_node(&mut config)
}

pub fn start_jormungandr_node_as_leader(mut config: &mut JormungandrConfig) -> ProcessKillGuard {
    jormungandr_starter::start_jormungandr_node_as_leader(&mut config)
}

pub fn build_configuration_with_funds(funds: Vec<Fund>) -> JormungandrConfig {
    let node_config = NodeConfig::new();
    let node_config_path = NodeConfig::serialize(&node_config);

    let genesis_model = GenesisYaml::new_with_funds(funds);
    let path_to_output_block = build_genesis_block(&genesis_model);

    let mut config = JormungandrConfig::from(genesis_model, node_config);

    let secret_key = jcli_wrapper::assert_key_generate_default();
    let secret_model = SecretModel::new(&secret_key);
    let secret_model_path = SecretModel::serialize(&secret_model);

    config.secret_model = secret_model;
    config.secret_model_path = secret_model_path;
    config.genesis_block_path = path_to_output_block.clone();
    config.node_config_path = node_config_path;

    config
}

pub fn build_configuration() -> JormungandrConfig {
    build_configuration_with_funds(vec![])
}

pub fn get_genesis_block_hash(genesis_yaml: &GenesisYaml) -> String {
    let path_to_output_block = build_genesis_block(&genesis_yaml);

    jcli_wrapper::assert_genesis_hash(&path_to_output_block)
}

pub fn build_genesis_block(genesis_yaml: &GenesisYaml) -> PathBuf {
    let input_yaml_file_path = GenesisYaml::serialize(&genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");

    println!(
        "output block file: {:?}, genesis_yaml {:?}",
        path_to_output_block, input_yaml_file_path
    );

    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);

    path_to_output_block
}

pub fn create_new_utxo_address() -> Utxo {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_single_default(&public_key);
    let utxo = Utxo {
        private_key,
        public_key,
        address,
    };
    println!("New utxo generated: {:?}", &utxo);
    utxo
}

pub fn create_new_account_address() -> Account {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_account_default(&public_key);
    let account = Account {
        private_key,
        public_key,
        address,
    };
    println!("New account generated: {:?}", &account);
    account
}

pub fn create_new_delegation_address() -> Delegation {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_single_default(&public_key);

    let private_delegation_key = jcli_wrapper::assert_key_generate_default();
    let public_delegation_key = jcli_wrapper::assert_key_to_public_default(&private_delegation_key);
    let delegation_address = jcli_wrapper::assert_address_single_default(&public_delegation_key);

    let utxo_with_delegation = Delegation {
        private_key,
        public_key,
        address,
        delegation_address,
    };
    println!(
        "New utxo with delegation generated: {:?}",
        &utxo_with_delegation
    );
    utxo_with_delegation
}

pub fn get_utxo_for_address<T: AddressDataProvider>(
    utxo_address: &T,
    jormungandr_rest_address: &str,
) -> UtxoData {
    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);
    utxos
        .into_iter()
        .find(|x| x.out_addr == utxo_address.get_address())
        .expect(&format!(
            "None utxo record found for {} of type({})",
            &utxo_address.get_address(),
            &utxo_address.get_address_type()
        ))
}
