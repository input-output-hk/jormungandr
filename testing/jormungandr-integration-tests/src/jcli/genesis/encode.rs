use crate::common::{
    configuration::{jormungandr_config::JormungandrConfig, Block0ConfigurationBuilder},
    file_assert, file_utils, jcli_wrapper,
    jormungandr::ConfigurationBuilder,
    startup,
};
use chain_addr::Discrimination;
use chain_impl_mockchain::fee::{LinearFee, PerCertificateFee, PerVoteCertificateFee};
use jormungandr_lib::interfaces::{Initial, InitialUTxO, LegacyUTxO};
use std::num::NonZeroU64;
#[test]
pub fn test_genesis_block_is_built_from_correct_yaml() {
    startup::build_genesis_block(&Block0ConfigurationBuilder::new().build());
}

#[test]
pub fn test_genesis_with_empty_consenus_leaders_list_fails_to_build() {
    let mut config: JormungandrConfig = Default::default();
    let mut block0_configuration = config.block0_configuration_mut();
    block0_configuration
        .blockchain_configuration
        .consensus_leader_ids = Vec::new();
    jcli_wrapper::assert_genesis_encode_fails(
        &config.block0_configuration(),
        r"Missing consensus leader id list in the initial fragment",
    );
}

#[test]
pub fn test_genesis_for_production_is_successfully_built() {
    let mut config: JormungandrConfig = Default::default();
    let mut block0_configuration = config.block0_configuration_mut();
    block0_configuration.initial.clear();
    block0_configuration.blockchain_configuration.discrimination = Discrimination::Production;
    let input_yaml_file_path = startup::serialize_block0_config(&config.block0_configuration());
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}

#[test]
pub fn test_genesis_for_prod_with_initial_funds_for_testing_address_fail_to_build() {
    let private_key = jcli_wrapper::assert_key_generate_default();
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let test_address = jcli_wrapper::assert_address_single(&public_key, Discrimination::Test);

    let mut config: JormungandrConfig = Default::default();
    let mut block0_configuration = config.block0_configuration_mut();
    block0_configuration.initial = vec![Initial::Fund(vec![InitialUTxO {
        value: 100.into(),
        address: test_address.parse().unwrap(),
    }])];
    block0_configuration.blockchain_configuration.discrimination = Discrimination::Production;
    jcli_wrapper::assert_genesis_encode_fails(
        config.block0_configuration(),
        "Invalid discrimination",
    );
}

#[test]
pub fn test_genesis_for_prod_with_wrong_discrimination_fail_to_build() {
    let mut config: JormungandrConfig = Default::default();
    let mut block0_configuration = config.block0_configuration_mut();
    block0_configuration.blockchain_configuration.discrimination = Discrimination::Production;
    jcli_wrapper::assert_genesis_encode_fails(
        config.block0_configuration(),
        "Invalid discrimination",
    );
}

#[test]
pub fn test_genesis_without_initial_funds_is_built_successfully() {
    let mut config: JormungandrConfig = Default::default();
    let block0_configuration = config.block0_configuration_mut();
    block0_configuration.initial.clear();
    let input_yaml_file_path = startup::serialize_block0_config(config.block0_configuration());
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}

#[test]
pub fn test_genesis_with_many_initial_funds_is_built_successfully() {
    let mut config: JormungandrConfig = Default::default();
    let address_1 = startup::create_new_account_address();
    let address_2 = startup::create_new_account_address();
    let initial_funds = Initial::Fund(vec![
        InitialUTxO {
            value: 100.into(),
            address: address_1.address(),
        },
        InitialUTxO {
            value: 100.into(),
            address: address_2.address(),
        },
    ]);
    let block0_configuration = config.block0_configuration_mut();
    block0_configuration.initial.push(initial_funds);
    let input_yaml_file_path = startup::serialize_block0_config(&config.block0_configuration());
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}

#[test]
pub fn test_genesis_with_legacy_funds_is_built_successfully() {
    let mut config: JormungandrConfig = Default::default();
    let legacy_funds = Initial::LegacyFund(
            vec![
                LegacyUTxO{
                    value: 100.into(),
                    address: "DdzFFzCqrht5TM5GznWhJ3GTpKawtJuA295F8igwXQXyt2ih1TL1XKnZqRBQBoLpyYVKfNKgCXPBUYruUneC83KjGK6QNAoBSqRJovbG".parse().unwrap()
                },
            ]
        );

    let block0_configuration = config.block0_configuration_mut();
    block0_configuration.initial.push(legacy_funds);
    let input_yaml_file_path = startup::serialize_block0_config(&config.block0_configuration());
    let path_to_output_block = file_utils::get_path_in_temp("block0.bin");
    jcli_wrapper::assert_genesis_encode(&input_yaml_file_path, &path_to_output_block);
}

#[test]
pub fn test_genesis_decode_bijection() {
    let mut fee = LinearFee::new(1, 1, 1);
    fee.per_certificate_fees(PerCertificateFee::new(
        Some(NonZeroU64::new(1).unwrap()),
        Some(NonZeroU64::new(1).unwrap()),
        Some(NonZeroU64::new(1).unwrap()),
    ));
    fee.per_vote_certificate_fees(PerVoteCertificateFee::new(
        Some(NonZeroU64::new(1).unwrap()),
        Some(NonZeroU64::new(1).unwrap()),
    ));
    let config = ConfigurationBuilder::new().with_linear_fees(fee).build();

    let expected_yaml_file_path = startup::serialize_block0_config(config.block0_configuration());
    let actual_yaml_file_path = file_utils::get_path_in_temp("actual_yaml.yaml");

    jcli_wrapper::assert_genesis_decode(&config.genesis_block_path(), &actual_yaml_file_path);
    file_assert::are_equal(&expected_yaml_file_path, &actual_yaml_file_path);

    let block0_after = file_utils::get_path_in_temp("block0_after.bin");
    jcli_wrapper::assert_genesis_encode(&actual_yaml_file_path, &block0_after);

    file_assert::are_equal(&config.genesis_block_path(), &block0_after);

    let right_hash = jcli_wrapper::assert_genesis_hash(&block0_after);

    assert_eq!(config.genesis_block_hash().clone(), right_hash);
}
