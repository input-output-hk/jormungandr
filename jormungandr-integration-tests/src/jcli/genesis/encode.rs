use crate::common::{
    configuration::{
        genesis_model::{Fund, GenesisYaml, Initial},
        jormungandr_config::JormungandrConfig,
    },
    file_utils, jcli_wrapper, startup,
};
use chain_addr::Discrimination;
use jormungandr_lib::interfaces::Value;

#[test]
pub fn test_genesis_block_is_built_from_corect_yaml() {
    startup::build_genesis_block(&GenesisYaml::new());
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
    config.genesis_yaml.initial.clear();
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
    config.genesis_yaml.initial = vec![Initial::Fund(vec![Fund {
        value: 100.into(),
        address: test_address.clone(),
    }])];
    config.genesis_yaml.blockchain_configuration.discrimination = Some("production".to_string());
    jcli_wrapper::assert_genesis_encode_fails(&config.genesis_yaml, "Invalid discrimination");
}

#[test]
pub fn test_genesis_for_prod_with_wrong_discrimination_fail_to_build() {
    let mut config = JormungandrConfig::new();
    config.genesis_yaml.blockchain_configuration.discrimination = Some("prod".to_string());
    jcli_wrapper::assert_genesis_encode_fails(
        &config.genesis_yaml,
        "blockchain_configuration.discrimination: unknown variant `prod`, expected `test` or `production`",
    );
}

#[test]
pub fn test_genesis_without_initial_funds_is_built_successfully() {
    let mut config = JormungandrConfig::new();
    config.genesis_yaml.initial.clear();
    let input_yaml_file_path = GenesisYaml::serialize(&config.genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}

#[test]
pub fn test_genesis_with_many_initial_funds_is_built_successfully() {
    let mut config = JormungandrConfig::new();
    let address_1 = startup::create_new_account_address();
    let address_2 = startup::create_new_account_address();
    let initial_funds = Initial::Fund(vec![
        Fund {
            value: 100.into(),
            address: address_1.address,
        },
        Fund {
            value: 100.into(),
            address: address_2.address,
        },
    ]);
    config.genesis_yaml.initial.push(initial_funds);
    let input_yaml_file_path = GenesisYaml::serialize(&config.genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}

#[test]
pub fn test_genesis_with_legacy_funds_is_built_successfully() {
    let mut config = JormungandrConfig::new();
    let legacy_funds = Initial::LegacyFund(
            vec![
                Fund{
                    value: 100.into(),
                    address: "DdzFFzCqrht5TM5GznWhJ3GTpKawtJuA295F8igwXQXyt2ih1TL1XKnZqRBQBoLpyYVKfNKgCXPBUYruUneC83KjGK6QNAoBSqRJovbG".to_string()
                },
            ]
        );
    config.genesis_yaml.initial.push(legacy_funds);
    let input_yaml_file_path = GenesisYaml::serialize(&config.genesis_yaml);
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}
