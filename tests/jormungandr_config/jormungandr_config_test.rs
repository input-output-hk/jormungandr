#![cfg(feature = "integration-test")]

use common::configuration::jormungandr_config::JormungandrConfig;
use common::configuration::node_config_model::Logger;
use common::startup;

#[test]
pub fn test_jormungandr_without_initial_funds_starts_sucessfully() {
    let mut config = JormungandrConfig::new();
    config.genesis_yaml.initial_funds = None;
    config.genesis_yaml.legacy_funds = None;
    let _jormungandr = startup::start_jormungandr_node(&mut config);
}

#[test]
pub fn test_jormungandr_with_empty_consenus_leaders_list_fails_to_start() {
    let mut config = JormungandrConfig::new();
    config
        .genesis_yaml
        .blockchain_configuration
        .consensus_leader_ids = Some(vec![]);

    startup::assert_jormungandr_node_fail_to_start(
        &mut config,
        r"Invalid blockchain state: Block0\(InitialMessageNoConsensusLeaderId\)",
    );
}

#[test]
pub fn test_jormungandr_with_no_trusted_peers_starts_succesfully() {
    let mut config = JormungandrConfig::new();
    config.node_config.peer_2_peer.trusted_peers = None;
    let _jormungandr = startup::start_jormungandr_node(&mut config);
    startup::assert_node_is_up(&config.get_node_address());
}

#[test]
pub fn test_jormungandr_with_wrong_logger_fails_to_start() {
    let mut config = JormungandrConfig::new();
    let logger = Logger {
        format: String::from("xml"),
        verbosity: 1,
    };
    config.node_config.logger = Some(logger);
    startup::assert_jormungandr_node_fail_to_start(
        &mut config,
        r"Error while parsing the node configuration file: logger\.format: unknown variant",
    );
}

#[test]
pub fn test_jormungandr_without_logger_starts_successfully() {
    let mut config = JormungandrConfig::new();
    config.node_config.logger = None;

    let _jormungandr = startup::start_jormungandr_node(&mut config);
    startup::assert_node_is_up(&config.get_node_address());
}
