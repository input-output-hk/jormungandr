use crate::common::configuration::node_config_model::{Logger, Peer};
use crate::common::startup;

#[test]
pub fn test_jormungandr_node_starts_successfully() {
    let mut config = startup::ConfigurationBuilder::new().build();
    let _jormungandr = startup::start_jormungandr_node(&mut config);
}

#[test]
pub fn test_jormungandr_leader_node_starts_successfully() {
    let mut config = startup::ConfigurationBuilder::new().build();
    let jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    jormungandr.assert_no_errors_in_log();
}

#[test]
#[ignore]
/// This test is wrong, it should extract the block0 created from the
/// first `leader_config` and only create a node configuration file
/// in the second passive node
pub fn test_jormungandr_passive_node_starts_successfully() {
    let mut leader_config = startup::ConfigurationBuilder::new().build();
    let _jormungandr_leader = startup::start_jormungandr_node_as_leader(&mut leader_config);

    let mut passive_config = startup::ConfigurationBuilder::new()
        .with_trusted_peers(vec![Peer {
            id: 1,
            address: leader_config.node_config.peer_2_peer.public_address.clone(),
        }])
        .build();

    let _jormungandr = startup::start_jormungandr_node_as_passive(&mut passive_config);
    startup::assert_node_is_up(&passive_config.get_node_address());
}

#[test]
pub fn test_jormungandr_passive_node_without_trusted_peers_fails_to_start() {
    let mut config = startup::ConfigurationBuilder::new()
        .with_trusted_peers(vec![])
        .build();
    let _jormungandr = startup::assert_start_jormungandr_node_as_passive_fail(
        &mut config,
        "no trusted peers specified",
    );
}

#[test]
pub fn test_jormungandr_without_initial_funds_starts_sucessfully() {
    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![])
        .build();
    config.genesis_yaml.legacy_funds = None;
    let _jormungandr = startup::start_jormungandr_node(&mut config);
}

#[test]
pub fn test_jormungandr_with_no_trusted_peers_starts_succesfully() {
    let mut config = startup::ConfigurationBuilder::new()
        .with_trusted_peers(vec![])
        .build();
    let _jormungandr = startup::start_jormungandr_node(&mut config);
    startup::assert_node_is_up(&config.get_node_address());
}

#[test]
pub fn test_jormungandr_with_wrong_logger_fails_to_start() {
    let mut config = startup::ConfigurationBuilder::new()
        .with_logger(Logger {
            format: String::from("xml"),
            verbosity: 1,
        })
        .build();
    startup::assert_start_jormungandr_node_as_passive_fail(
        &mut config,
        r"Error while parsing the node configuration file: logger\.format: unknown variant",
    );
}

#[test]
pub fn test_jormungandr_without_logger_starts_successfully() {
    let mut config = startup::ConfigurationBuilder::new().build();
    config.node_config.logger = None;
    let _jormungandr = startup::start_jormungandr_node(&mut config);
    startup::assert_node_is_up(&config.get_node_address());
}
