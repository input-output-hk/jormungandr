use crate::common::configuration::node_config_model::{Log, TrustedPeer};
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
pub fn test_jormungandr_passive_node_starts_successfully() {
    let mut leader_config = startup::ConfigurationBuilder::new().build();
    let _jormungandr_leader = startup::start_jormungandr_node_as_leader(&mut leader_config);

    let mut passive_config = startup::ConfigurationBuilder::new()
        .with_trusted_peers(vec![TrustedPeer {
            address: leader_config.node_config.p2p.public_address.clone(),
            id: leader_config.public_id.clone(),
        }])
        .with_block_hash(leader_config.genesis_block_hash)
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
    let mut config = startup::ConfigurationBuilder::new().build();
    config.genesis_yaml.initial.clear();
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
        .with_log(Log {
            format: Some("xml".to_string()),
            level: None,
        })
        .build();
    startup::assert_start_jormungandr_node_as_passive_fail(
        &mut config,
        r"Error while parsing the node configuration file: log\.format: unknown variant",
    );
}

#[test]
pub fn test_jormungandr_without_logger_starts_successfully() {
    let mut config = startup::ConfigurationBuilder::new().build();
    config.node_config.log = None;
    let _jormungandr = startup::start_jormungandr_node(&mut config);
    startup::assert_node_is_up(&config.get_node_address());
}
