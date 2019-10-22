use crate::common::{
    configuration::node_config_model::{Log, TrustedPeer},
    jormungandr::{ConfigurationBuilder, Starter},
};

#[test]
pub fn test_jormungandr_leader_node_starts_successfully() {
    let jormungandr = Starter::new().start().unwrap();
    jormungandr.assert_no_errors_in_log();
}

#[test]
pub fn test_jormungandr_passive_node_starts_successfully() {
    let leader_config = ConfigurationBuilder::new().build();
    let _jormungandr_leader = Starter::new()
        .config(leader_config.clone())
        .start()
        .unwrap();

    let passive_config = ConfigurationBuilder::new()
        .with_trusted_peers(vec![TrustedPeer {
            address: leader_config.node_config.p2p.public_address.clone(),
            id: leader_config.public_id.clone(),
        }])
        .with_block_hash(leader_config.genesis_block_hash)
        .build();

    let _jormungandr_passive = Starter::new().config(passive_config).start().unwrap();
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
    config.genesis_yaml.initial.clear();
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
        .with_log(Log {
            format: Some("xml".to_string()),
            level: None,
        })
        .build();
    Starter::new().config(config).start_fail(
        r"Error while parsing the node configuration file: log\.format: unknown variant",
    );
}

#[test]
pub fn test_jormungandr_without_logger_starts_successfully() {
    let mut config = ConfigurationBuilder::new().build();
    config.node_config.log = None;
    let _jormungandr = Starter::new().config(config).start().unwrap();
}
