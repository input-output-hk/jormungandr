use assert_fs::{prelude::*, TempDir};
use jormungandr_automation::jormungandr::{ConfigurationBuilder, Starter};
use jormungandr_lib::interfaces::{Log, LogEntry, LogOutput};

#[test]
pub fn test_jormungandr_leader_node_starts_successfully() {
    let jormungandr = Starter::new().start().unwrap();
    jormungandr.assert_no_errors_in_log();
}

#[test]
pub fn test_jormungandr_passive_node_starts_successfully() {
    let temp_dir = TempDir::new().unwrap();

    let leader_dir = temp_dir.child("leader");
    leader_dir.create_dir_all().unwrap();
    let leader_config = ConfigurationBuilder::new().build(&leader_dir);
    let jormungandr_leader = Starter::new()
        .config(leader_config.clone())
        .start()
        .unwrap();

    let passive_dir = temp_dir.child("passive");
    passive_dir.create_dir_all().unwrap();
    let passive_config = ConfigurationBuilder::new()
        .with_trusted_peers(vec![jormungandr_leader.to_trusted_peer()])
        .with_block_hash(leader_config.genesis_block_hash())
        .build(&passive_dir);

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
    let temp_dir = TempDir::new().unwrap();

    let config = ConfigurationBuilder::new()
        .with_trusted_peers(vec![])
        .build(&temp_dir);

    Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .passive()
        .start_fail("no trusted peers specified")
}

#[test]
pub fn test_jormungandr_without_initial_funds_starts_sucessfully() {
    let temp_dir = TempDir::new().unwrap();
    let mut config = ConfigurationBuilder::new().build(&temp_dir);
    let block0_configuration = config.block0_configuration_mut();
    block0_configuration.initial.clear();
    let _jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();
}

#[test]
pub fn test_jormungandr_with_no_trusted_peers_starts_succesfully() {
    let temp_dir = TempDir::new().unwrap();
    let config = ConfigurationBuilder::new()
        .with_trusted_peers(vec![])
        .build(&temp_dir);
    let _jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();
}

#[test]
pub fn test_jormungandr_with_wrong_logger_fails_to_start() {
    let temp_dir = TempDir::new().unwrap();
    let config = ConfigurationBuilder::new()
        .with_log(Log(LogEntry {
            format: "xml".to_string(),
            level: "info".to_string(),
            output: LogOutput::Stderr,
        }))
        .build(&temp_dir);
    Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start_fail(r"Error in the overall configuration of the node");
}

#[test]
pub fn test_jormungandr_without_logger_starts_successfully() {
    let temp_dir = TempDir::new().unwrap();
    let config = ConfigurationBuilder::new().without_log().build(&temp_dir);
    let _jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();
}
