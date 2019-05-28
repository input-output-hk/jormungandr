#![cfg(feature = "integration-test")]

use common::configuration::genesis_model::GenesisYaml;

use common::configuration::genesis_model::Fund;
use common::configuration::jormungandr_config::JormungandrConfig;
use common::file_utils;
use common::jcli_wrapper;
use common::startup;

#[test]
pub fn test_genesis_block_is_built_from_corect_yaml() {
    startup::build_genesis_block(&GenesisYaml::new());
}

#[test]
pub fn test_genesis_without_block0_date_fails_to_build() {
    let mut config = JormungandrConfig::new();
    config.genesis_yaml.blockchain_configuration.block0_date = None;
    jcli_wrapper::assert_genesis_encode_fails(&config.genesis_yaml, "missing field `block0_date`")
}

#[test]
pub fn test_genesis_with_empty_consenus_leaders_list_fails_to_build() {
    let mut config = JormungandrConfig::new();
    config
        .genesis_yaml
        .blockchain_configuration
        .consensus_leader_ids = Some(vec![]);
    jcli_wrapper::assert_genesis_encode_fails(
        &config.genesis_yaml,
        r"Block0\(InitialMessageNoConsensusLeaderId\)",
    );
}

#[test]
pub fn test_genesis_for_production_is_successfully_built() {
    let mut config = JormungandrConfig::new();
    config.genesis_yaml.initial_funds = None;
    config.genesis_yaml.blockchain_configuration.discrimination = Some("production".to_string());
    let input_yaml_file_path = GenesisYaml::serialize(&config.genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}

#[test]
pub fn test_genesis_for_prod_with_initial_funds_for_testing_address_fail_to_build() {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let test_address = jcli_wrapper::assert_address_single_for_testing(&public_key);

    let mut config = JormungandrConfig::new();
    config.genesis_yaml.initial_funds = Some(vec![Fund {
        value: 100,
        address: test_address.clone(),
    }]);
    config.genesis_yaml.blockchain_configuration.discrimination = Some("production".to_string());
    jcli_wrapper::assert_genesis_encode_fails(&config.genesis_yaml, "InvalidDiscrimination");
}

#[test]
pub fn test_genesis_for_prod_with_wrong_discrimination_fail_to_build() {
    let mut config = JormungandrConfig::new();
    config.genesis_yaml.blockchain_configuration.discrimination = Some("prod".to_string());
    jcli_wrapper::assert_genesis_encode_fails(
        &config.genesis_yaml,
        " Invalid Address Discrimination",
    );
}

#[test]
pub fn test_genesis_without_initial_funds_is_built_successfully() {
    let mut config = JormungandrConfig::new();
    config.genesis_yaml.initial_funds = None;
    let input_yaml_file_path = GenesisYaml::serialize(&config.genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}
