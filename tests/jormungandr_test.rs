#![cfg(feature = "integration-test")]

mod common;
use common::configuration::node_config_model::Peer;
use common::startup;

#[test]
pub fn test_jormungandr_node_starts_successfully() {
    let mut config = startup::ConfigurationBuilder::new().build();
    let _jormungandr = startup::start_jormungandr_node(&mut config);
}

#[test]
pub fn test_jormungandr_leader_node_starts_successfully() {
    let mut config = JormungandrConfig::new();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
}

#[test]
pub fn test_jormungandr_passive_node_starts_successfully() {
    let mut leader_config = JormungandrConfig::new();
    let _jormungandr_leader = startup::start_jormungandr_node_as_leader(&mut leader_config);

    let mut passive_config = JormungandrConfig::new();
    passive_config.node_config.peer_2_peer.trusted_peers = Some(vec![Peer {
        id: 1,
        address: leader_config.node_config.peer_2_peer.public_address.clone(),
    }]);

    let _jormungandr = startup::start_jormungandr_node_as_passive(&mut passive_config);
    startup::assert_node_is_up(&passive_config.get_node_address());
}

#[test]
pub fn test_jormungandr_passive_node_without_trusted_peers_fails_to_start() {
    let mut config = JormungandrConfig::new();
    config.node_config.peer_2_peer.trusted_peers = None;
    let _jormungandr = startup::assert_start_jormungandr_node_as_passive_fail(
        &mut config,
        "no trusted peers specified",
    );
}
