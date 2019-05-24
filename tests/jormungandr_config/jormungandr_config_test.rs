#![cfg(feature = "integration-test")]

use common::configuration::node_config_model::Logger;
use common::startup;

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
