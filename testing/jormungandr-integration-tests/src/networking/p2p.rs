use crate::common::{
    jormungandr::process::JormungandrProcess,
    network::{self, wallet},
};

use jormungandr_lib::{
    interfaces::{
        Explorer, PeerRecord, Policy, PreferredListConfig, TopicsOfInterest, TrustedPeer,
    },
    time::Duration,
};
use jormungandr_testing_utils::testing::network_builder::SpawnParams;
use jortestkit::process as process_utils;
const CLIENT: &str = "CLIENT";
const SERVER: &str = "SERVER";

pub fn assert_empty_quarantine(node: &JormungandrProcess, info: &str) {
    let quarantine = node
        .rest()
        .p2p_quarantined()
        .expect("cannot list quarantined peers");
    assert!(
        quarantine.is_empty(),
        "{}: Peer {} has got non empty quarantine list",
        info,
        node.alias()
    );
}

pub fn assert_are_in_quarantine(
    node: &JormungandrProcess,
    peers: Vec<&JormungandrProcess>,
    info: &str,
) {
    let available_list = node
        .rest()
        .p2p_quarantined()
        .expect("cannot list quarantined peers");
    assert_record_is_present(available_list, peers, "quarantine", info)
}

pub fn assert_record_is_present(
    peer_list: Vec<PeerRecord>,
    peers: Vec<&JormungandrProcess>,
    list_name: &str,
    info: &str,
) {
    for peer in peers {
        assert!(
            peer_list.iter().any(|x| {
                println!("{} == {}", x.address, peer.address().to_string());
                x.address == peer.address().to_string()
            }),
            "{}: Peer {} is not present in {} list",
            info,
            peer.alias(),
            list_name
        );
    }
}

pub fn assert_record_is_not_present(
    peer_list: Vec<PeerRecord>,
    peers: Vec<&JormungandrProcess>,
    list_name: &str,
) {
    for peer in peers {
        assert!(
            !peer_list
                .iter()
                .any(|x| { x.address == peer.address().to_string() }),
            "Peer {} is present in {} list, while should not",
            peer.alias(),
            list_name
        );
    }
}

pub fn assert_node_stats(
    node: &JormungandrProcess,
    peer_available_cnt: usize,
    peer_quarantined_cnt: usize,
    peer_total_cnt: usize,
    info: &str,
) {
    node.log_stats();
    let stats = node
        .rest()
        .stats()
        .expect("cannot get stats")
        .stats
        .expect("empty stats");
    assert_eq!(
        peer_available_cnt,
        stats.peer_available_cnt.clone(),
        "{}: peer_available_cnt, Node {}",
        info,
        node.alias()
    );

    assert_eq!(
        peer_quarantined_cnt,
        stats.peer_quarantined_cnt,
        "{}: peer_quarantined_cnt, Node {}",
        info,
        node.alias()
    );
    assert_eq!(
        peer_total_cnt,
        stats.peer_total_cnt,
        "{}: peer_total_cnt, Node {}",
        info,
        node.alias()
    );
}

#[test]
pub fn node_whitelist_itself() {
    let mut network_controller = network::builder()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            wallet("delegated1").with(1_000_000).delegated_to(CLIENT),
            wallet("delegated2").with(1_000_000).delegated_to(SERVER),
        ])
        .custom_config(vec![
            SpawnParams::new(CLIENT).explorer(Explorer { enabled: true })
        ])
        .build()
        .unwrap();

    let _server = network_controller.spawn_and_wait(SERVER);

    let client_public_address = network_controller
        .node_config(CLIENT)
        .unwrap()
        .p2p
        .public_address;
    let policy = Policy {
        quarantine_duration: Some(Duration::new(1, 0)),
        quarantine_whitelist: Some(vec![client_public_address]),
    };

    let client = network_controller
        .spawn_custom(network_controller.spawn_params(CLIENT).policy(policy))
        .unwrap();
    client.assert_no_errors_in_log();
}

#[test]
pub fn node_does_not_quarantine_whitelisted_node() {
    let mut network_controller = network::builder()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            wallet("delegated1").with(1_000_000).delegated_to(CLIENT),
            wallet("delegated2").with(1_000_000).delegated_to(SERVER),
        ])
        .custom_config(vec![
            SpawnParams::new(CLIENT).explorer(Explorer { enabled: true })
        ])
        .build()
        .unwrap();

    let fake_addr = "/ip4/127.0.0.1/tcp/80";
    let policy = Policy {
        quarantine_duration: Some(Duration::new(30, 0)),
        quarantine_whitelist: Some(vec![fake_addr.parse().unwrap()]),
    };

    let server = network_controller
        .spawn_custom(network_controller.spawn_params(SERVER).policy(policy))
        .unwrap();

    let _client = network_controller
        .spawn_custom(
            network_controller
                .spawn_params(CLIENT)
                // The client broadcast a different ip address from the one it's actually
                // listening to, so that the server will fail connection
                .public_address("/ip4/127.0.0.1/tcp/80".parse().unwrap())
                .listen_address(Some(
                    network_controller
                        .node_config(CLIENT)
                        .unwrap()
                        .p2p
                        .get_listen_addr()
                        .unwrap(),
                )),
        )
        .unwrap();

    assert_node_stats(&server, 1, 0, 1, "after starting client");
    assert_empty_quarantine(&server, "after starting client");
}

#[test]
pub fn node_put_in_quarantine_nodes_which_are_not_whitelisted() {
    let mut network_controller = network::builder()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            wallet("delegated1").with(1_000_000).delegated_to(CLIENT),
            wallet("delegated2").with(1_000_000).delegated_to(SERVER),
        ])
        .custom_config(vec![
            SpawnParams::new(CLIENT).explorer(Explorer { enabled: true })
        ])
        .build()
        .unwrap();

    let server = network_controller.spawn_and_wait(SERVER);

    let client = network_controller
        .spawn_custom(
            network_controller
                .spawn_params(CLIENT)
                // The client broadcast a different ip address from the one it's actually
                // listening to, so that the server will fail connection and put it in quarantine
                .public_address("/ip4/127.0.0.1/tcp/80".parse().unwrap())
                .listen_address(Some(
                    network_controller
                        .node_config(CLIENT)
                        .unwrap()
                        .p2p
                        .get_listen_addr()
                        .unwrap(),
                )),
        )
        .unwrap();

    process_utils::sleep(5);

    assert_node_stats(&server, 0, 1, 1, "after starting client");
    assert_are_in_quarantine(&server, vec![&client], "after starting client");
}

#[test]
pub fn node_does_not_quarantine_trusted_node() {
    let mut network_controller = network::builder()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            wallet("delegated1").with(1_000_000).delegated_to(CLIENT),
            wallet("delegated2").with(1_000_000).delegated_to(SERVER),
        ])
        .custom_config(vec![
            SpawnParams::new(CLIENT).explorer(Explorer { enabled: true })
        ])
        .build()
        .unwrap();

    let server = network_controller.spawn_and_wait(SERVER);
    let client = network_controller.spawn_and_wait(CLIENT);

    process_utils::sleep(5);

    assert_node_stats(&server, 1, 0, 1, "before stopping client");
    assert_empty_quarantine(&server, "before stopping client");

    client.shutdown();
    process_utils::sleep(20);

    // The server "forgets" the client but does not quarantine it
    assert_node_stats(&server, 0, 0, 0, "before restarting client");
    assert_empty_quarantine(&server, "before restarting client");
}

#[test]
pub fn node_trust_itself() {
    let mut network_controller = network::builder()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            wallet("delegated1").with(1_000_000).delegated_to(CLIENT),
            wallet("delegated2").with(1_000_000).delegated_to(SERVER),
        ])
        .custom_config(vec![
            SpawnParams::new(CLIENT).explorer(Explorer { enabled: true })
        ])
        .build()
        .unwrap();

    let _server = network_controller.spawn_and_wait(SERVER);

    let config = network_controller.node_config(CLIENT).unwrap().p2p;

    let peer = TrustedPeer {
        address: config.public_address,
        id: None,
    };
    network_controller
        .expect_spawn_failed(
            network_controller
                .spawn_params(CLIENT)
                .trusted_peers(vec![peer]),
            "failed to retrieve the list of bootstrap peers from trusted peer",
        )
        .unwrap();
}

#[test]
#[ignore]
pub fn node_put_itself_in_preffered_layers() {
    let mut network_controller = network::builder()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            wallet("delegated1").with(1_000_000).delegated_to(CLIENT),
            wallet("delegated2").with(1_000_000).delegated_to(SERVER),
        ])
        .build()
        .unwrap();

    let _server = network_controller.spawn_and_wait(SERVER);

    let config = network_controller.node_config(CLIENT).unwrap().p2p;

    let peer = TrustedPeer {
        address: config.public_address,
        id: None,
    };

    let layer = PreferredListConfig {
        view_max: Default::default(),
        peers: vec![peer],
    };

    assert!(network_controller
        .expect_spawn_failed(
            network_controller
                .spawn_params(CLIENT)
                .preferred_layer(layer),
            "topology tells the node to connect to itself"
        )
        .is_ok());
}

#[ignore]
#[test]
pub fn topic_of_interest_influences_node_sync_ability() {
    let fast_client_alias = "FAST_CLIENT";
    let slow_client_alias = "SLOW_CLIENT";

    let high_topic_of_interests = TopicsOfInterest {
        messages: "high".to_owned(),
        blocks: "high".to_owned(),
    };

    let low_topic_of_interests = TopicsOfInterest {
        messages: "low".to_owned(),
        blocks: "low".to_owned(),
    };

    let mut network_controller = network::builder()
        .star_topology(SERVER, vec![fast_client_alias, slow_client_alias])
        .initials(vec![
            wallet("delegated0").with(1_000_000).delegated_to(SERVER),
            wallet("delegated1")
                .with(1_000_000)
                .delegated_to(fast_client_alias),
            wallet("delegated2")
                .with(1_000_000)
                .delegated_to(slow_client_alias),
        ])
        .custom_config(vec![
            SpawnParams::new(fast_client_alias).topics_of_interest(high_topic_of_interests),
            SpawnParams::new(slow_client_alias).topics_of_interest(low_topic_of_interests),
        ])
        .build()
        .unwrap();

    let _server = network_controller.spawn_and_wait(SERVER);
    let fast_client = network_controller.spawn_and_wait(fast_client_alias);
    let slow_client = network_controller.spawn_and_wait(fast_client_alias);

    process_utils::sleep(30);

    let fast_client_block_recv_cnt = fast_client
        .rest()
        .stats()
        .unwrap()
        .stats
        .unwrap()
        .block_recv_cnt;
    let slow_client_block_recv_cnt = slow_client
        .rest()
        .stats()
        .unwrap()
        .stats
        .unwrap()
        .block_recv_cnt;

    assert!(
        fast_client_block_recv_cnt > slow_client_block_recv_cnt,
        "node with high block topic of interest should have more recieved blocks"
    );
}
