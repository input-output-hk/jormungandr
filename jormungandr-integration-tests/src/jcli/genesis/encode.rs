use crate::common::{
    configuration::{jormungandr_config::JormungandrConfig, Block0ConfigurationBuilder},
    file_utils, jcli_wrapper, startup,
};
use chain_addr::Discrimination;
use jormungandr_lib::interfaces::{Initial, InitialUTxO, LegacyUTxO};

#[test]
pub fn test_genesis_block_is_built_from_correct_yaml() {
    startup::build_genesis_block(&Block0ConfigurationBuilder::new().build());
}

#[test]
pub fn test_genesis_with_empty_consenus_leaders_list_fails_to_build() {
    let mut config = JormungandrConfig::new();
    config
        .block0_configuration
        .blockchain_configuration
        .consensus_leader_ids = vec![];
    jcli_wrapper::assert_genesis_encode_fails(
        &config.block0_configuration,
        r"Missing consensus leader id list in the initial fragment",
    );
}

#[test]
pub fn test_genesis_for_production_is_successfully_built() {
    let mut config = JormungandrConfig::new();
    config.block0_configuration.initial.clear();
    config
        .block0_configuration
        .blockchain_configuration
        .discrimination = Discrimination::Production;
    let input_yaml_file_path = startup::serialize_block0_config(&config.block0_configuration);
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}

#[test]
pub fn test_genesis_for_prod_with_initial_funds_for_testing_address_fail_to_build() {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let test_address = jcli_wrapper::assert_address_single(&public_key, Discrimination::Test);

    let mut config = JormungandrConfig::new();
    config.block0_configuration.initial = vec![Initial::Fund(vec![InitialUTxO {
        value: 100.into(),
        address: test_address.parse().unwrap(),
    }])];
    config
        .block0_configuration
        .blockchain_configuration
        .discrimination = Discrimination::Production;
    jcli_wrapper::assert_genesis_encode_fails(
        &config.block0_configuration,
        "Invalid discrimination",
    );
}

#[test]
pub fn test_genesis_for_prod_with_wrong_discrimination_fail_to_build() {
    let mut config = JormungandrConfig::new();
    config
        .block0_configuration
        .blockchain_configuration
        .discrimination = Discrimination::Production;
    jcli_wrapper::assert_genesis_encode_fails(
        &config.block0_configuration,
        "Invalid discrimination",
    );
}

#[test]
pub fn test_genesis_without_initial_funds_is_built_successfully() {
    let mut config = JormungandrConfig::new();
    config.block0_configuration.initial.clear();
    let input_yaml_file_path = startup::serialize_block0_config(&config.block0_configuration);
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}

#[test]
pub fn test_genesis_with_many_initial_funds_is_built_successfully() {
    let mut config = JormungandrConfig::new();
    let address_1 = startup::create_new_account_address();
    let address_2 = startup::create_new_account_address();
    let initial_funds = Initial::Fund(vec![
        InitialUTxO {
            value: 100.into(),
            address: address_1.address.parse().unwrap(),
        },
        InitialUTxO {
            value: 100.into(),
            address: address_2.address.parse().unwrap(),
        },
    ]);
    config.block0_configuration.initial.push(initial_funds);
    let input_yaml_file_path = startup::serialize_block0_config(&config.block0_configuration);
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}

#[test]
pub fn test_genesis_with_legacy_funds_is_built_successfully() {
    let mut config = JormungandrConfig::new();
    let legacy_funds = Initial::LegacyFund(
            vec![
                LegacyUTxO{
                    value: 100.into(),
                    address: "DdzFFzCqrht5TM5GznWhJ3GTpKawtJuA295F8igwXQXyt2ih1TL1XKnZqRBQBoLpyYVKfNKgCXPBUYruUneC83KjGK6QNAoBSqRJovbG".parse().unwrap()
                },
            ]
        );
    config.block0_configuration.initial.push(legacy_funds);
    let input_yaml_file_path = startup::serialize_block0_config(&config.block0_configuration);
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}
