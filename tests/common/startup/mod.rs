use std::path::PathBuf;

use super::configuration::genesis_model::GenesisYaml;
use super::configuration::node_config_model::NodeConfig;
use super::file_utils;
use super::jcli_wrapper;
use super::jormungandr_wrapper;
use super::process_utils;
use super::process_utils::process_guard::ProcessKillGuard;

pub fn start_jormungandr_node_and_wait(
    config_path: &PathBuf,
    genesis_block_path: &PathBuf,
) -> ProcessKillGuard {
    println!("Starting jormungandr node...");

    let process = jormungandr_wrapper::start_jormungandr_node(&config_path, &genesis_block_path)
        .spawn()
        .expect("failed to execute 'start jormungandr node'");
    let guard = ProcessKillGuard::new(process, String::from("Jormungandr node"));

    process_utils::run_process_until_exited_successfully(
        jcli_wrapper::get_rest_stats_command_default(),
        2,
        5,
        "get stats from jormungandr node",
        "jormungandr node is not up",
    );
    println!("Jormungandr node started");

    guard
}

pub fn start_jormungandr_node_with_genesis_conf(genesis_yaml: GenesisYaml) -> ProcessKillGuard {
    let path_to_output_block = build_genesis_block(genesis_yaml);
    let node_config_path = NodeConfig::serialize(NodeConfig::new());
    let process = start_jormungandr_node_and_wait(&node_config_path, &path_to_output_block);
    process
}

pub fn build_genesis_block(genesis_yaml: GenesisYaml) -> PathBuf {
    let input_yaml_file_path = GenesisYaml::serialize(genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");

    println!(
        "output block file: {:?}, genesis_yaml {:?}",
        path_to_output_block, input_yaml_file_path
    );

    jcli_wrapper::assert_genesis_encode_command(&input_yaml_file_path, &path_to_output_block);

    path_to_output_block
}
