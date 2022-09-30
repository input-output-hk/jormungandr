use assert_fs::{fixture::ChildPath, prelude::*, TempDir};
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    fee::{LinearFee, PerCertificateFee, PerVoteCertificateFee},
    vote::CommitteeId,
};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{Block0ConfigurationBuilder, ConfigurationBuilder, JormungandrParams},
};
use jormungandr_lib::interfaces::{Block0Configuration, Initial, InitialUTxO, LegacyUTxO};
use std::{num::NonZeroU64, path::Path};

struct Fixture {
    temp_dir: TempDir,
    params: JormungandrParams,
}

impl Fixture {
    fn new() -> Self {
        Self::with_config(&ConfigurationBuilder::new())
    }

    fn with_config(builder: &ConfigurationBuilder) -> Self {
        let temp_dir = TempDir::new().unwrap();
        let params = builder.build(&temp_dir);
        Fixture { temp_dir, params }
    }

    fn temp_dir(&self) -> &TempDir {
        &self.temp_dir
    }

    fn jormungandr_params(&self) -> &JormungandrParams {
        &self.params
    }

    fn config(&self) -> &Block0Configuration {
        self.params.block0_configuration()
    }

    fn config_mut(&mut self) -> &mut Block0Configuration {
        self.params.block0_configuration_mut()
    }

    fn write_config(&self, file_name: impl AsRef<Path>) -> ChildPath {
        let yaml_file = self.temp_dir.child(file_name);
        let content = serde_yaml::to_string(self.config()).unwrap();
        yaml_file.write_str(&content).unwrap();
        yaml_file
    }

    fn assert_encode(&self) {
        let yaml_file = self.write_config("genesis.yaml");
        let jcli: JCli = Default::default();
        jcli.genesis()
            .encode(yaml_file.path(), &self.temp_dir.child("block-0.bin"));
    }

    fn assert_encode_fails(&self, expected_msg: &str) {
        let jcli: JCli = Default::default();

        let yaml_file = self.write_config("genesis.yaml");

        jcli.genesis()
            .encode_expect_fail(yaml_file.path(), expected_msg);
    }
}

#[test]
pub fn test_genesis_block_is_built_from_correct_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let config = Block0ConfigurationBuilder::new().build();
    let config_file = temp_dir.child("genesis.yaml");
    let content = serde_yaml::to_string(&config).unwrap();
    config_file.write_str(&content).unwrap();
    let output_block_file = temp_dir.child("block-0.bin");
    let jcli: JCli = Default::default();
    jcli.genesis()
        .encode(config_file.path(), &output_block_file);

    assert!(output_block_file.path().exists());
}

#[test]
pub fn test_genesis_with_empty_consenus_leaders_list_fails_to_build() {
    let mut fixture = Fixture::new();
    let block0_configuration = fixture.config_mut();
    block0_configuration
        .blockchain_configuration
        .consensus_leader_ids = Vec::new();
    fixture.assert_encode_fails(r"Missing consensus leader id list in the initial fragment")
}

#[test]
pub fn test_genesis_for_production_is_successfully_built() {
    let mut fixture = Fixture::new();
    let block0_configuration = fixture.config_mut();
    block0_configuration.initial.clear();
    block0_configuration.blockchain_configuration.discrimination = Discrimination::Production;
    fixture.assert_encode()
}

#[test]
pub fn test_genesis_for_prod_with_initial_funds_for_testing_address_fail_to_build() {
    let jcli: JCli = Default::default();

    let private_key = jcli.key().generate_default();
    let public_key = jcli.key().convert_to_public_string(&private_key);
    let test_address = jcli
        .address()
        .single(&public_key, None, Discrimination::Test);

    let mut fixture = Fixture::new();
    let block0_configuration = fixture.config_mut();
    block0_configuration.initial = vec![Initial::Fund(vec![InitialUTxO {
        value: 100.into(),
        address: test_address.parse().unwrap(),
    }])];
    block0_configuration.blockchain_configuration.discrimination = Discrimination::Production;
    fixture.assert_encode_fails("Invalid discrimination")
}

#[test]
pub fn test_genesis_for_prod_with_wrong_discrimination_fail_to_build() {
    let mut fixture = Fixture::new();
    let block0_configuration = fixture.config_mut();
    block0_configuration.blockchain_configuration.discrimination = Discrimination::Production;
    fixture.assert_encode_fails("Invalid discrimination")
}

#[test]
pub fn test_genesis_without_initial_funds_is_built_successfully() {
    let mut fixture = Fixture::new();
    let block0_configuration = fixture.config_mut();
    block0_configuration.initial.clear();
    fixture.assert_encode()
}

#[test]
pub fn test_genesis_with_many_initial_funds_is_built_successfully() {
    let address_1 = thor::Wallet::default();
    let address_2 = thor::Wallet::default();
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
    let mut fixture = Fixture::new();
    let block0_configuration = fixture.config_mut();
    block0_configuration.initial.push(initial_funds);
    fixture.assert_encode()
}

#[test]
pub fn test_genesis_with_legacy_funds_is_built_successfully() {
    let legacy_funds = Initial::LegacyFund(
            vec![
                LegacyUTxO{
                    value: 100.into(),
                    address: "DdzFFzCqrht5TM5GznWhJ3GTpKawtJuA295F8igwXQXyt2ih1TL1XKnZqRBQBoLpyYVKfNKgCXPBUYruUneC83KjGK6QNAoBSqRJovbG".parse().unwrap()
                },
            ]
        );
    let mut fixture = Fixture::new();
    let block0_configuration = fixture.config_mut();
    block0_configuration.initial.push(legacy_funds);
    fixture.assert_encode()
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
    let fixture = Fixture::with_config(ConfigurationBuilder::new().with_linear_fees(fee));
    let params = fixture.jormungandr_params();

    let expected_yaml_file = fixture.write_config("expected-genesis.yaml");
    let actual_yaml_file = fixture.temp_dir().child("actual-genesis.yaml");
    let jcli: JCli = Default::default();

    jcli.genesis()
        .decode(params.genesis_block_path(), &actual_yaml_file);
    actual_yaml_file.assert(jortestkit::prelude::file_text_content_is_same_as(
        expected_yaml_file.path(),
    ));

    let block0_after = fixture.temp_dir().child("block-0-after.bin");
    jcli.genesis()
        .encode(actual_yaml_file.path(), &block0_after);

    block0_after.assert(jortestkit::prelude::file_binary_content_is_same_as(
        params.genesis_block_path(),
    ));

    let right_hash = jcli.genesis().hash(block0_after.path());

    assert_eq!(params.genesis_block_hash(), right_hash.to_string());
}

#[test]
pub fn test_encode_genesis_with_vit_params() {
    let mut fee = LinearFee::new(1, 1, 1);
    fee.per_vote_certificate_fees(PerVoteCertificateFee::new(
        Some(NonZeroU64::new(2).unwrap()),
        Some(NonZeroU64::new(1).unwrap()),
    ));
    let fixture = Fixture::with_config(
        ConfigurationBuilder::new()
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
            ]),
    );
    fixture.assert_encode();
}
