#![allow(dead_code)]

use super::configuration::genesis_model::GenesisYaml;
use super::configuration::node_config_model::NodeConfig;
use super::configuration::secret_model::SecretModel;
use super::data::address::{Account, Delegation, Utxo};
use super::data::utxo::Utxo as UtxoData;
use super::file_utils;
use super::jcli_wrapper;
use super::jormungandr_wrapper;
use super::process_utils;
use super::process_utils::output_extensions::ProcessOutput;
use super::process_utils::process_guard::ProcessKillGuard;
use common::configuration::genesis_model::Fund;
use common::configuration::jormungandr_config::JormungandrConfig;
use std::path::PathBuf;
use std::process::Command;

pub fn from_initial_funds(funds: Vec<Fund>) -> JormungandrConfig {
    let node_config = NodeConfig::new();
    let genesis_model = GenesisYaml::new_with_funds(funds);
    let config = JormungandrConfig::from(genesis_model, node_config);
    config
}

fn start_jormungandr_node_and_wait(rest_address: &str, mut command: Command) -> ProcessKillGuard {
    println!("Starting jormungandr node...");
    let process = command
        .spawn()
        .expect("failed to execute 'start jormungandr node'");

    let guard = ProcessKillGuard::new(process, String::from("Jormungandr node"));

    process_utils::run_process_until_response_matches(
        jcli_wrapper::jcli_commands::get_rest_stats_command(&rest_address),
        |output| match output.as_single_node_yaml().get("uptime") {
            Some(uptime) => {
                uptime
                    .parse::<i32>()
                    .expect(&format!("Cannot parse uptime {}", uptime.to_string()))
                    > 2
            }
            None => false,
        },
        2,
        5,
        "get stats from jormungandr node",
        "jormungandr node is not up",
    );
    println!("Jormungandr node started");

    guard
}

pub fn start_jormungandr_node(config: &mut JormungandrConfig) -> ProcessKillGuard {
    let rest_address = &config.node_config.get_node_address();
    let config_path = NodeConfig::serialize(&config.node_config);
    let path_to_output_block = build_genesis_block(&config.genesis_yaml);

    let command = jormungandr_wrapper::get_start_jormungandr_node_command(
        &config_path,
        &path_to_output_block,
    );

    config.genesis_block_path = path_to_output_block.clone();
    println!("Starting node with configuration : {:?}", &config);

    let process = start_jormungandr_node_and_wait(&rest_address, command);
    process
}

pub fn start_jormungandr_node_as_leader(config: &mut JormungandrConfig) -> ProcessKillGuard {
    let rest_address = &config.node_config.get_node_address();
    let config_path = NodeConfig::serialize(&config.node_config);
    let genesis_block_path = build_genesis_block(&config.genesis_yaml);
    let secret_key = jcli_wrapper::assert_key_generate_default();
    let secret_model = SecretModel::new(&secret_key);
    let secret_model_path = SecretModel::serialize(&secret_model);

    let command = jormungandr_wrapper::get_start_jormungandr_as_leader_node_command(
        &config_path,
        &genesis_block_path,
        &secret_model_path,
    );

    config.genesis_block_path = genesis_block_path.clone();
    println!("Starting node with configuration : {:?}", &config);
    let process = start_jormungandr_node_and_wait(&rest_address, command);
    process
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

pub fn get_utxo_for_address(utxo_address: &Utxo, jormungandr_rest_address: &str) -> UtxoData {
    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);
    utxos
        .into_iter()
        .find(|x| x.out_addr == utxo_address.address)
        .expect(&format!("None utxo record found for {:?}", &utxo_address))
}
