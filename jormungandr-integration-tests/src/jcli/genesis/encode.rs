use crate::common::configuration::{
    genesis_model::{Fund, GenesisYaml},
    jormungandr_config::JormungandrConfig,
};
use crate::common::file_utils;
use crate::common::jcli_wrapper;
use crate::common::jcli_wrapper::Discrimination;
use crate::common::startup;

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
        r"Missing consensus leader id list in the initial fragment",
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
    let test_address = jcli_wrapper::assert_address_single(&public_key, Discrimination::Test);

    let mut config = JormungandrConfig::new();
    config.genesis_yaml.initial_funds = Some(vec![Fund {
        value: 100,
        address: test_address.clone(),
    }]);
    config.genesis_yaml.blockchain_configuration.discrimination = Some("production".to_string());
    jcli_wrapper::assert_genesis_encode_fails(&config.genesis_yaml, "Invalid discrimination");
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

#[test]
pub fn test_genesis_block_is_built_for_praos_consensus() {
    let leader = startup::create_new_utxo_address();

    let config = startup::ConfigurationBuilder::new()
        .with_block0_consensus("genesis")
        .with_bft_slots_ratio("0".to_owned())
        .with_consensus_genesis_praos_active_slot_coeff("0.1")
        .with_consensus_leaders_ids(vec![leader.public_key.clone()])
        .with_kes_update_speed(43200)
        .build();

    let input_yaml_file_path = GenesisYaml::serialize(&config.genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}

#[test]
pub fn genesis_block_is_not_build_from_non_existent_input_file() {
    let config = startup::ConfigurationBuilder::new().build();
    let mut input_yaml_file_path = GenesisYaml::serialize(&config.genesis_yaml);
    input_yaml_file_path.push("a");
    jcli_wrapper::assert_genesis_encode_from_file_fails(
        &input_yaml_file_path,
        "invalid input file path",
    );
}

#[test]
pub fn genesis_block_is_not_build_from_file_not_in_yaml_format() {
    let invalid_content = "blockchain_configuration:\
                           block0_date: 1550822014\
                           discrimination: test\
                           block0_consensus: bft\
                           slots_per_epoch: 5\
                           epoch_stability_depth: 10\
                           consensus_genesis_praos_active_slot_coeff: 0.22\
                           consensus_leader_ids:\
                           - ed25519e_pk1k3wjgdcdcn23k6dwr0cyh88ad7a4ayenyxaherfazwy363pyy8wqppn7j3\
                           - ed25519e_pk13talprd9grgaqzs42mkm0x2xek5wf9mdf0eefdy8a6dk5grka2gstrp3en\
                           linear_fees:\
                           constant: 0aasa\
                           coefficient: 0asas\
                           certificate: 0";
    let input_yaml_file_path =
        file_utils::create_file_in_temp("incorrect_genesis.yaml", &invalid_content);
    jcli_wrapper::assert_genesis_encode_from_file_fails(
        &input_yaml_file_path,
        "genesis file corrupted",
    );
}

#[test]
pub fn genesis_block_is_build_from_legacy_funds() {
    let mut config = startup::ConfigurationBuilder::new().build();
    config.genesis_yaml.legacy_funds = Some(vec![Fund {
        address: "Ae2tdPwUPEZCEhYAUVU7evPfQCJjyuwM6n81x6hSjU9TBMSy2YwZEVydssL".to_string(),
        value: 2000,
    }]);
    let input_yaml_file_path = GenesisYaml::serialize(&config.genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}
