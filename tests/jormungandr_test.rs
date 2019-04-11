extern crate assert_cmd;
extern crate galvanic_test;
extern crate mktemp;

mod common;
use common::configuration;
use common::file_assert;
use common::file_utils;
use common::jcli_wrapper;
use common::jormungandr_wrapper;
use common::process_assert;
use common::process_utils;

#[test]
#[cfg(feature = "integration-test")]
pub fn test_jormungandr_node_starts_successfully() {
    let genesis_yaml = configuration::genesis_model::GenesisYaml::new();
    let content = serde_yaml::to_string(&genesis_yaml).unwrap();
    let input_yaml_file_path = file_utils::create_file_in_temp("genesis.yaml", &content);

    let node_config = configuration::get_node_config_path();
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");

    process_assert::run_and_assert_process_exited_successfully(
        jcli_wrapper::get_genesis_encode_command(&input_yaml_file_path, &path_to_output_block),
        "jcli genesis encode",
    );

    file_assert::assert_file_exists(&path_to_output_block);

    let process = jormungandr_wrapper::start_jormungandr_node(&node_config, &path_to_output_block)
        .spawn()
        .expect("failed to execute 'start jormungandr node'");
    let _guard = process_utils::process_guard::ProcessKillGuard::new(process);

    process_utils::run_process_until_exited_successfully(
        jcli_wrapper::get_rest_stats_command_default(),
        2,
        5,
        "get stats from jormungandr node",
        "jormungandr node is not up",
    );
}
