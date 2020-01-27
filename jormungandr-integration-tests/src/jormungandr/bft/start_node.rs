use crate::common::jormungandr::{ConfigurationBuilder, Starter};

use jormungandr_lib::interfaces::{Log, LogEntry, LogOutput, TrustedPeer};

#[test]
pub fn test_jormungandr_leader_node_starts_successfully() {
    let jormungandr = Starter::new().start().unwrap();
    jormungandr.assert_no_errors_in_log();
}

#[test]
pub fn test_jormungandr_passive_node_starts_successfully() {
    let leader_config = ConfigurationBuilder::new().build();
    let jormungandr_leader = Starter::new()
        .config(leader_config.clone())
        .start()
        .unwrap();

    let passive_config = ConfigurationBuilder::new()
        .with_trusted_peers(vec![jormungandr_leader.as_trusted_peer()])
        .with_block_hash(leader_config.genesis_block_hash)
        .build();

    let jormungandr_passive = Starter::new()
        .config(passive_config)
        .passive()
        .start()
        .unwrap();
    jormungandr_passive.assert_no_errors_in_log();
    jormungandr_leader.assert_no_errors_in_log();
}

#[test]
pub fn test_jormungandr_passive_node_without_trusted_peers_fails_to_start() {
    let config = ConfigurationBuilder::new()
        .with_trusted_peers(vec![])
        .build();

    Starter::new()
        .config(config)
        .passive()
        .start_fail("no trusted peers specified")
}

#[test]
pub fn test_jormungandr_without_initial_funds_starts_sucessfully() {
    let mut config = ConfigurationBuilder::new().build();
    config.block0_configuration.initial.clear();
    let _jormungandr = Starter::new().config(config).start().unwrap();
}

#[test]
pub fn test_jormungandr_with_no_trusted_peers_starts_succesfully() {
    let config = ConfigurationBuilder::new()
        .with_trusted_peers(vec![])
        .build();
    let _jormungandr = Starter::new().config(config).start().unwrap();
}

#[test]
pub fn test_jormungandr_with_wrong_logger_fails_to_start() {
    let config = ConfigurationBuilder::new()
        .with_log(Log(vec![LogEntry {
            format: "xml".to_string(),
            level: "info".to_string(),
            output: LogOutput::Stderr,
        }]))
        .build();
    Starter::new().config(config).start_fail(
        r"Error while parsing the node configuration file: log\[0\]\.format: unknown variant",
    );
}

#[test]
pub fn test_jormungandr_without_logger_starts_successfully() {
    let mut config = ConfigurationBuilder::new().build();
    config.node_config.log = None;
    let _jormungandr = Starter::new().config(config).start().unwrap();
}
