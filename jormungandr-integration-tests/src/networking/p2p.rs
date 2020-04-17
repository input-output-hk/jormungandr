use crate::common::{
    network::{builder, params, wallet, Node},
    process_utils,
};
use jormungandr_lib::interfaces::Explorer;
use jormungandr_lib::{
    interfaces::{PeerRecord, Policy},
    time::Duration,
};
const CLIENT: &str = "CLIENT";
const SERVER: &str = "SERVER";

pub fn assert_empty_quarantine(node: &Node, info: &str) {
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

pub fn assert_are_in_quarantine(node: &Node, peers: Vec<&Node>, info: &str) {
    let available_list = node
        .rest()
        .p2p_quarantined()
        .expect("cannot list quarantined peers");
    assert_record_is_present(available_list, peers, "quarantine", info)
}

pub fn assert_record_is_present(
    peer_list: Vec<PeerRecord>,
    peers: Vec<&Node>,
    list_name: &str,
    info: &str,
) {
    for peer in peers {
        assert!(
            peer_list.iter().any(|x| {
                let info = &x.profile.info;
                println!(
                    "{} == {} , {} == {}",
                    info.id,
                    peer.public_id().to_string(),
                    info.address,
                    peer.address().to_string()
                );
                info.id == peer.public_id().to_string()
                    && info.address == peer.address().to_string()
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
    peers: Vec<&Node>,
    list_name: &str,
) {
    for peer in peers {
        assert!(
            !peer_list.iter().any(|x| {
                let info = &x.profile.info;
                info.id == peer.public_id().to_string()
                    && info.address == peer.address().to_string()
            }),
            "Peer {} is present in {} list, while should not",
            peer.alias(),
            list_name
        );
    }
}

pub fn assert_node_stats(
    node: &Node,
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
    let mut network_controller = builder("node_whitelist_itself")
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
        quarantine_whitelist: Some(vec![client_public_address.clone()]),
    };

    let client = network_controller
        .spawn_custom(params(CLIENT).policy(policy))
        .unwrap();
    client.assert_no_errors_in_log();
}

#[test]
pub fn node_does_not_quarantine_whitelisted_node() {
    let mut network_controller = builder("node_whitelist_itself")
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            wallet("delegated1").with(1_000_000).delegated_to(CLIENT),
            wallet("delegated2").with(1_000_000).delegated_to(SERVER),
        ])
        .custom_config(vec![params(CLIENT).explorer(Explorer { enabled: true })])
        .build()
        .unwrap();

    let mut server = network_controller.spawn_and_wait(SERVER);

    let server_public_address = network_controller
        .node_config(SERVER)
        .unwrap()
        .p2p
        .public_address;
    let policy = Policy {
        quarantine_duration: Some(Duration::new(30, 0)),
        quarantine_whitelist: Some(vec![server_public_address.clone()]),
    };

    let client = network_controller
        .spawn_custom(params(CLIENT).policy(policy))
        .unwrap();

    server.shutdown();

    process_utils::sleep(10);

    assert_node_stats(&client, 1, 0, 1, 0, "before spawning server again");
    assert_empty_quarantine(&client, "before spawning server again");

    server = network_controller.spawn_and_wait(SERVER);

    assert_node_stats(&client, 1, 0, 1, 0, "after spawning server again");
    assert_empty_quarantine(&client, "after spawning server again");
}

#[test]
pub fn node_put_in_quarantine_nodes_which_are_not_whitelisted() {
    let mut network_controller = builder("node_whitelist_itself")
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
        quarantine_whitelist: Some(vec![client_public_address.clone()]),
    };

    let client = network_controller
        .spawn_custom(params(CLIENT).policy(policy))
        .unwrap();

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
