use crate::startup::SingleNodeTestBootstrapper;
use assert_fs::TempDir;
use jormungandr_automation::{
    jormungandr::{Block0ConfigurationBuilder, JormungandrBootstrapper, NodeConfigBuilder},
    testing::block0::Block0ConfigurationExtension,
};
use jormungandr_lib::interfaces::{Log, LogEntry, LogOutput};

#[test]
pub fn test_jormungandr_leader_node_starts_successfully() {
    let temp_dir = TempDir::new().unwrap();

    let jormungandr = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();
    jormungandr.assert_no_errors_in_log();
}

#[test]
pub fn test_jormungandr_passive_node_starts_successfully() {
    let leader_temp_dir = TempDir::new().unwrap();
    let passive_temp_dir = TempDir::new().unwrap();

    let test_context = SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .build();
    let jormungandr_leader = test_context.start_node(leader_temp_dir).unwrap();

    let jormungandr_passive = JormungandrBootstrapper::default()
        .passive()
        .with_block0_hash(test_context.block0_config().to_block_hash())
        .with_node_config(
            NodeConfigBuilder::default()
                .with_trusted_peers(vec![jormungandr_leader.to_trusted_peer()])
                .build(),
        )
        .start(passive_temp_dir)
        .unwrap();

    jormungandr_passive.assert_no_errors_in_log();
    jormungandr_leader.assert_no_errors_in_log();
}

#[test]
pub fn test_jormungandr_passive_node_without_trusted_peers_fails_to_start() {
    let temp_dir = TempDir::new().unwrap();

    let block0 = Block0ConfigurationBuilder::minimal_setup().build();

    JormungandrBootstrapper::default()
        .passive()
        .with_block0_hash(block0.to_block_hash())
        .with_node_config(
            NodeConfigBuilder::default()
                .with_trusted_peers(vec![])
                .build(),
        )
        .into_starter(temp_dir)
        .unwrap()
        .start_should_fail_with_message("no trusted peers specified")
        .unwrap();
}

#[test]
pub fn test_jormungandr_without_initial_funds_starts_sucessfully() {
    let temp_dir = TempDir::new().unwrap();
    SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();
}

#[test]
pub fn test_jormungandr_with_no_trusted_peers_starts_succesfully() {
    let temp_dir = TempDir::new().unwrap();
    SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .build()
        .start_node(temp_dir)
        .unwrap();
}

#[test]
pub fn test_jormungandr_with_wrong_logger_fails_to_start() {
    let temp_dir = TempDir::new().unwrap();

    SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_node_config(NodeConfigBuilder::default().with_log(Log(LogEntry {
            format: "xml".to_string(),
            level: "info".to_string(),
            output: LogOutput::Stderr,
        })))
        .build()
        .starter(temp_dir)
        .unwrap()
        .start_should_fail_with_message(r"Error in the overall configuration of the node")
        .unwrap();
}

#[test]
pub fn test_jormungandr_without_logger_starts_successfully() {
    let temp_dir = TempDir::new().unwrap();
    SingleNodeTestBootstrapper::default()
        .as_bft_leader()
        .with_node_config(NodeConfigBuilder::default().without_log())
        .build()
        .start_node(temp_dir)
        .unwrap();
}
