extern crate assert_cmd;
extern crate galvanic_test;
extern crate mktemp;

mod common;
use common::configuration;
use common::file_assert;
use common::file_utils;
use common::jcli_wrapper;
use common::process_assert;

#[test]
#[cfg(feature = "integration-test")]
pub fn test_genesis_block_is_built_from_corect_yaml() {
    let genesis_yaml = configuration::genesis_model::GenesisYaml::new();
    let content = serde_yaml::to_string(&genesis_yaml).unwrap();
    let input_yaml_file_path = file_utils::create_file_in_temp("genesis.yaml", &content);
    let path_to_output_block = file_utils::get_path_in_temp("block-0.bin");
    println!(
        "output block file: {:?}, genesis_yaml {:?}",
        path_to_output_block, input_yaml_file_path
    );

    process_assert::run_and_assert_process_exited_successfully(
        jcli_wrapper::get_genesis_encode_command(&input_yaml_file_path, &path_to_output_block),
        "jcli genesis encode",
    );
    file_assert::assert_file_exists_and_not_empty(path_to_output_block);
}
