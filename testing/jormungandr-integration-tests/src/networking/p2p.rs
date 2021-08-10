use crate::common::{
    jormungandr::process::JormungandrProcess,
    network::{NetworkBuilder, WalletTemplateBuilder},
};

use jormungandr_lib::{
    interfaces::{PeerRecord, Policy, PreferredListConfig, TrustedPeer},
    time::Duration,
};
use jormungandr_testing_utils::testing::FragmentNode;
use jortestkit::process as process_utils;
use tracing::Level;

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
    let mut network_controller = NetworkBuilder::default()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            WalletTemplateBuilder::new("delegated1")
                .with(1_000_000)
                .delegated_to(CLIENT),
            WalletTemplateBuilder::new("delegated2")
                .with(1_000_000)
                .delegated_to(SERVER),
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
    let mut network_controller = NetworkBuilder::default()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            WalletTemplateBuilder::new("delegated1")
                .with(1_000_000)
                .delegated_to(CLIENT),
            WalletTemplateBuilder::new("delegated2")
                .with(1_000_000)
                .delegated_to(SERVER),
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

    assert_node_stats(&server, 1, 0, 1, 0, "after starting client");
    assert_empty_quarantine(&server, "after starting client");
}

#[test]
pub fn node_put_in_quarantine_nodes_which_are_not_whitelisted() {
    let mut network_controller = NetworkBuilder::default()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            WalletTemplateBuilder::new("delegated1")
                .with(1_000_000)
                .delegated_to(CLIENT),
            WalletTemplateBuilder::new("delegated2")
                .with(1_000_000)
                .delegated_to(SERVER),
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

    assert_node_stats(&server, 0, 1, 1, 0, "after starting client");
    assert_are_in_quarantine(&server, vec![&client], "after starting client");
}

#[test]
pub fn node_does_not_quarantine_trusted_node() {
    let mut network_controller = NetworkBuilder::default()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            WalletTemplateBuilder::new("delegated1")
                .with(1_000_000)
                .delegated_to(CLIENT),
            WalletTemplateBuilder::new("delegated2")
                .with(1_000_000)
                .delegated_to(SERVER),
        ])
        .build()
        .unwrap();

    let server = network_controller.spawn_and_wait(SERVER);
    let client = network_controller.spawn_and_wait(CLIENT);

    assert_node_stats(&server, 1, 0, 1, 0, "before stopping client");
    assert_empty_quarantine(&server, "before stopping client");

    client.shutdown();
    process_utils::sleep(20);

    // The server "forgets" the client but does not quarantine it
    assert_node_stats(&server, 0, 0, 0, 0, "before restarting client");
    assert_empty_quarantine(&server, "before restarting client");
}

#[test]
pub fn node_trust_itself() {
    let mut network_controller = NetworkBuilder::default()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            WalletTemplateBuilder::new("delegated1")
                .with(1_000_000)
                .delegated_to(CLIENT),
            WalletTemplateBuilder::new("delegated2")
                .with(1_000_000)
                .delegated_to(SERVER),
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
    let mut network_controller = NetworkBuilder::default()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            WalletTemplateBuilder::new("delegated1")
                .with(1_000_000)
                .delegated_to(CLIENT),
            WalletTemplateBuilder::new("delegated2")
                .with(1_000_000)
                .delegated_to(SERVER),
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

#[test]
fn gossip_interval() {
    const INTERVAL_SECS: u64 = 3;

    let mut network_controller = NetworkBuilder::default()
        .single_trust_direction(CLIENT, SERVER)
        .initials(vec![
            WalletTemplateBuilder::new("delegated1")
                .with(1_000_000)
                .delegated_to(CLIENT),
            WalletTemplateBuilder::new("delegated2")
                .with(1_000_000)
                .delegated_to(SERVER),
        ])
        .build()
        .unwrap();

    let _server = network_controller
        .spawn_custom(
            network_controller
                .spawn_params(SERVER)
                .gossip_interval(Duration::new(INTERVAL_SECS, 0)),
        )
        .unwrap();

    let client = network_controller
        .spawn_custom(
            network_controller
                .spawn_params(CLIENT)
                .log_level(Level::TRACE),
        )
        .unwrap();

    process_utils::sleep(10);

    let log_timestamps: Vec<u64> = client
        .log_content()
        .into_iter()
        .filter(|s| s.contains("accept_gossips"))
        .skip(1)
        .map(|t| parse_timestamp(&t))
        .collect();

    let mut prev = None;

    for log_timestamp in log_timestamps {
        match prev {
            None => prev = Some(log_timestamp),
            Some(prev) => {
                assert!(log_timestamp - prev >= INTERVAL_SECS);
            }
        }
    }
}

#[cfg(feature = "soak-non-functional")]
#[test]
fn network_stuck_check() {
    const INTERVAL_SECS: u64 = 90;
    let mut network_controller = NetworkBuilder::default()
        .single_trust_direction(CLIENT, SERVER)
        .build()
        .unwrap();

    let server = network_controller.spawn_and_wait(SERVER);

    let client = network_controller
        .spawn_custom(
            network_controller
                .spawn_params(CLIENT)
                .log_level(Level::TRACE)
                .gossip_interval(Duration::new(5, 0))
                .network_stuck_check(Duration::new(INTERVAL_SECS, 0)),
        )
        .unwrap();

    server.stop();

    process_utils::sleep(10 * INTERVAL_SECS);

    let log_timestamps: Vec<u64> = client
        .log_content()
        .into_iter()
        .filter(|s| s.contains("p2p network have been too quiet for some time"))
        .map(|t| parse_timestamp(&t))
        .collect();

    let mut prev = None;

    for log_timestamp in log_timestamps {
        match prev {
            None => prev = Some(log_timestamp),
            Some(prev) => {
                assert!(log_timestamp - prev >= INTERVAL_SECS);
            }
        }
    }
}

#[test]
fn max_bootstrap_attempts() {
    const ATTEMPTS: usize = 3;

    let mut network_controller = NetworkBuilder::default()
        .single_trust_direction(CLIENT, SERVER)
        .build()
        .unwrap();

    let client = network_controller
        .spawn_custom(
            network_controller
                .spawn_params(CLIENT)
                .max_bootstrap_attempts(ATTEMPTS)
                .log_level(Level::TRACE),
        )
        .unwrap();

    process_utils::sleep(5);

    assert_eq!(
        client
            .log_content()
            .into_iter()
            .filter(|l| l.contains("bootstrap attempt #"))
            .count(),
        ATTEMPTS
    );
}

fn parse_timestamp(log: &str) -> u64 {
    let re = regex::Regex::new("([0-9]+):([0-9]+):([0-9]+)").unwrap();

    let captures = re.captures(log).unwrap();

    let mut seconds = 0;

    for i in 1..=3 {
        seconds +=
            captures.get(i).unwrap().as_str().parse::<u64>().unwrap() * 60_u64.pow(3 - i as u32);
    }

    seconds
}

#[test]
fn log_parser() {
    assert_eq!(parse_timestamp("00:00:00"), 0);
    assert_eq!(parse_timestamp("00:00:42"), 42);
    assert_eq!(parse_timestamp("00:01:13"), 60 + 13);
    assert_eq!(parse_timestamp("00:45:12"), 45 * 60 + 12);
    assert_eq!(parse_timestamp("01:34:02"), 3600 + 34 * 60 + 2);
    assert_eq!(parse_timestamp("10:02:31"), 10 * 3600 + 2 * 60 + 31);
}
