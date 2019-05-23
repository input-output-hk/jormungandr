#![cfg(feature = "integration-test")]

use common::configuration::genesis_model::GenesisYaml;
use common::configuration::jormungandr_config::JormungandrConfig;
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
