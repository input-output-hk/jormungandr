use assert_fs::{fixture::ChildPath, prelude::*, TempDir};
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    fee::{LinearFee, PerCertificateFee, PerVoteCertificateFee},
    vote::CommitteeId,
};
use jormungandr_automation::{jcli::JCli, jormungandr::Block0ConfigurationBuilder};
use jormungandr_lib::interfaces::{Block0Configuration, Initial, InitialUTxO, LegacyUTxO};
use rand::thread_rng;
use std::num::NonZeroU64;
use thor::Block0ConfigurationBuilderExtension;

fn write_genesis_yaml(
    block0_configuration: &Block0Configuration,
    yaml_file: ChildPath,
) -> ChildPath {
    let content = serde_yaml::to_string(block0_configuration).unwrap();
    yaml_file.write_str(&content).unwrap();
    yaml_file
}

fn assert_encode(block0_configuration: &Block0Configuration, temp_dir: TempDir) {
    let yaml_file = temp_dir.child("genesis.yaml");
    let block_0 = temp_dir.child("block0.bin");
    let yaml_file = write_genesis_yaml(block0_configuration, yaml_file);
    let jcli: JCli = Default::default();
    jcli.genesis().encode(yaml_file.path(), &block_0);
}

fn assert_encode_fails(
    block0_configuration: &Block0Configuration,
    temp_dir: TempDir,
    expected_msg: &str,
) {
    let jcli: JCli = Default::default();

    let yaml_file = write_genesis_yaml(block0_configuration, temp_dir.child("genesis.yaml"));

    jcli.genesis()
        .encode_expect_fail(yaml_file.path(), expected_msg);
}

#[test]
pub fn test_genesis_block_is_built_from_correct_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let config = Block0ConfigurationBuilder::minimal_setup().build();
    let config_file = write_genesis_yaml(&config, temp_dir.child("genesis.yaml"));
    let output_block_file = temp_dir.child("block-0.bin");
    let jcli: JCli = Default::default();
    jcli.genesis()
        .encode(config_file.path(), &output_block_file);
    assert!(output_block_file.path().exists());
}

#[test]
pub fn test_genesis_with_empty_consenus_leaders_list_fails_to_build() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = Block0ConfigurationBuilder::default().build();
    config.blockchain_configuration.consensus_leader_ids = Vec::new();
    assert_encode_fails(
        &config,
        temp_dir,
        r"Missing consensus leader id list in the initial fragment",
    );
}

#[test]
pub fn test_genesis_for_production_is_successfully_built() {
    let temp_dir = TempDir::new().unwrap();

    assert_encode(
        &Block0ConfigurationBuilder::minimal_setup()
            .with_discrimination(Discrimination::Test)
            .build(),
        temp_dir,
    );
}

#[test]
pub fn test_genesis_for_prod_with_wrong_discrimination_fail_to_build() {
    let temp_dir = TempDir::new().unwrap();
    let wallet = thor::Wallet::new_account(&mut thread_rng(), Discrimination::Test);
    let block0_configuration = Block0ConfigurationBuilder::minimal_setup()
        .with_discrimination(Discrimination::Production)
        .with_wallets_having_some_values(vec![&wallet])
        .build();

    assert_encode_fails(&block0_configuration, temp_dir, "Invalid discrimination");
}

#[test]
pub fn test_genesis_without_initial_funds_is_built_successfully() {
    let temp_dir = TempDir::new().unwrap();
    let block0_config = Block0ConfigurationBuilder::default()
        .with_some_consensus_leader()
        .build();
    assert_encode(&block0_config, temp_dir);
}

#[test]
pub fn test_genesis_with_many_initial_funds_is_built_successfully() {
    let temp_dir = TempDir::new().unwrap();

    let address_1 = thor::Wallet::default();
    let address_2 = thor::Wallet::default();
    let initial_funds = vec![
        InitialUTxO {
            value: 100.into(),
            address: address_1.address(),
        },
        InitialUTxO {
            value: 100.into(),
            address: address_2.address(),
        },
    ];

    let block0_config = Block0ConfigurationBuilder::minimal_setup()
        .with_utxos(initial_funds)
        .build();
    assert_encode(&block0_config, temp_dir)
}

#[test]
pub fn test_genesis_with_legacy_funds_is_built_successfully() {
    let temp_dir = TempDir::new().unwrap();

    let legacy_funds = Initial::LegacyFund(
            vec![
                LegacyUTxO{
                    value: 100.into(),
                    address: "DdzFFzCqrht5TM5GznWhJ3GTpKawtJuA295F8igwXQXyt2ih1TL1XKnZqRBQBoLpyYVKfNKgCXPBUYruUneC83KjGK6QNAoBSqRJovbG".parse().unwrap()
                },
            ]
        );

    let block0_config = Block0ConfigurationBuilder::minimal_setup()
        .with_initial(vec![legacy_funds])
        .build();
    assert_encode(&block0_config, temp_dir)
}

#[test]
pub fn test_genesis_decode_bijection() {
    let temp_dir = TempDir::new().unwrap();
    let jcli = JCli::default();

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

    let block_config = Block0ConfigurationBuilder::minimal_setup()
        .with_linear_fees(fee)
        .build();
    let expected_yaml_file = temp_dir.child("initial_genesis.yaml");
    let expected_block0_file = temp_dir.child("initial_block0.bin");

    expected_yaml_file
        .write_str(&serde_yaml::to_string(&block_config).unwrap())
        .unwrap();

    jcli.genesis()
        .encode(expected_yaml_file.path(), &expected_block0_file);

    let actual_yaml_file = temp_dir.child("actual_genesis.yaml");

    jcli.genesis()
        .decode(expected_block0_file.path(), &actual_yaml_file);

    expected_yaml_file.assert(jortestkit::prelude::file_text_content_is_same_as(
        actual_yaml_file.path(),
    ));

    let actual_block0_file = temp_dir.child("actual_block0.bin");
    jcli.genesis()
        .encode(actual_yaml_file.path(), &actual_block0_file);

    expected_block0_file.assert(jortestkit::prelude::file_binary_content_is_same_as(
        actual_block0_file.path(),
    ));

    assert_eq!(
        jcli.genesis().hash(actual_block0_file.path()),
        jcli.genesis().hash(expected_block0_file.path())
    );
}

#[test]
pub fn test_encode_genesis_with_vit_params() {
    let temp_dir = TempDir::new().unwrap();

    let mut fee = LinearFee::new(1, 1, 1);
    fee.per_vote_certificate_fees(PerVoteCertificateFee::new(
        Some(NonZeroU64::new(2).unwrap()),
        Some(NonZeroU64::new(1).unwrap()),
    ));

    let block0_config = Block0ConfigurationBuilder::minimal_setup()
        .with_linear_fees(fee)
        .with_committee_ids(vec![
            CommitteeId::from_hex(
                "7ef044ba437057d6d944ace679b7f811335639a689064cd969dffc8b55a7cc19",
            )
            .unwrap()
            .into(),
            CommitteeId::from_hex(
                "f5285eeead8b5885a1420800de14b0d1960db1a990a6c2f7b517125bedc000db",
            )
            .unwrap()
            .into(),
        ])
        .build();

    assert_encode(&block0_config, temp_dir);
}
