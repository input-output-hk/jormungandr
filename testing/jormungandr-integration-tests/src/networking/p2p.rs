use crate::common::{
    jormungandr::process::JormungandrProcess,
    network::{self, params, wallet},
};

use jormungandr_lib::{
    interfaces::{
        Explorer, LayersConfig, PeerRecord, Policy, PreferredListConfig, TopicsOfInterest,
        TrustedPeer,
    },
    time::Duration,
};
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
    peer_unreachable_cnt: usize,
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
    assert_eq!(
        peer_unreachable_cnt,
        stats.peer_unreachable_cnt,
        "{}: peer_unreachable_cnt, Node {}",
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
        .custom_config(vec![params(CLIENT).explorer(Explorer { enabled: true })])
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
        .spawn_custom(params(CLIENT).policy(policy))
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
        .custom_config(vec![params(CLIENT).explorer(Explorer { enabled: true })])
        .build()
        .unwrap();

    let server = network_controller.spawn_and_wait(SERVER);

    let server_public_address = network_controller
        .node_config(SERVER)
        .unwrap()
        .p2p
        .public_address;
    let policy = Policy {
        quarantine_duration: Some(Duration::new(30, 0)),
        quarantine_whitelist: Some(vec![server_public_address]),
    };

    let client = network_controller
        .spawn_custom(params(CLIENT).policy(policy))
        .unwrap();

    // Give time to the client to accept incoming gossip from the server and
    // add it to its topology
    process_utils::sleep(5);
    server.shutdown();

    process_utils::sleep(10);

    assert_node_stats(&client, 1, 0, 1, 0, "before spawning server again");
    assert_empty_quarantine(&client, "before spawning server again");

    let _server_after = network_controller.spawn_and_wait(SERVER);

    assert_node_stats(&client, 1, 0, 1, 0, "after spawning server again");
    assert_empty_quarantine(&client, "after spawning server again");
}

#[test]
pub fn node_put_in_quarantine_nodes_which_are_not_whitelisted() {
    let mut network_controller = network::builder()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            wallet("delegated1").with(1_000_000).delegated_to(CLIENT),
            wallet("delegated2").with(1_000_000).delegated_to(SERVER),
        ])
        .custom_config(vec![params(CLIENT).explorer(Explorer { enabled: true })])
        .build()
        .unwrap();

    let mut server = network_controller.spawn_and_wait(SERVER);

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
        .spawn_custom(params(CLIENT).policy(policy))
        .unwrap();

    // Give time to the client to accept incoming gossip from the server and
    // add it to its topology
    process_utils::sleep(5);
    server.shutdown();

    process_utils::sleep(10);

    assert_node_stats(&client, 0, 1, 1, 0, "before spawning server again");
    assert_are_in_quarantine(&client, vec![&server], "before spawning server again");

    server = network_controller.spawn_and_wait(SERVER);

    assert_node_stats(&client, 0, 1, 1, 0, "after spawning server again");
    assert_are_in_quarantine(&client, vec![&server], "after spawning server again");

    process_utils::sleep(10);

    assert_node_stats(
        &client,
        0,
        1,
        1,
        0,
        "after spawning server again (10 s. delay)",
    );
    assert_are_in_quarantine(
        &client,
        vec![&server],
        "after spawning server again (10 s. delay)",
    );
}

#[test]
pub fn node_trust_itself() {
    let mut network_controller = network::builder()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            wallet("delegated1").with(1_000_000).delegated_to(CLIENT),
            wallet("delegated2").with(1_000_000).delegated_to(SERVER),
        ])
        .custom_config(vec![params(CLIENT).explorer(Explorer { enabled: true })])
        .build()
        .unwrap();

    let _server = network_controller.spawn_and_wait(SERVER);

    let config = network_controller.node_config(CLIENT).unwrap().p2p.clone();

    let peer = TrustedPeer {
        address: config.public_address,
        id: None,
    };

    assert!(network_controller
        .expect_spawn_failed(
            params(CLIENT).trusted_peers(vec![peer]),
            "unable to reach peer for initial bootstrap"
        )
        .is_ok());
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

    let config = network_controller.node_config(CLIENT).unwrap().p2p.clone();

    let peer = TrustedPeer {
        address: config.public_address,
        id: None,
    };

    let layer = LayersConfig {
        preferred_list: PreferredListConfig {
            view_max: Default::default(),
            peers: vec![peer],
        },
    };

    assert!(network_controller
        .expect_spawn_failed(
            params(CLIENT).preferred_layer(layer),
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
            params(fast_client_alias).topics_of_interest(high_topic_of_interests),
            params(slow_client_alias).topics_of_interest(low_topic_of_interests),
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
