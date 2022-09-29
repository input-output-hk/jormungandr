use crate::networking::utils;
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{Blockchain, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::jormungandr::LogLevel;
use jormungandr_lib::{
    interfaces::{TopicsOfInterest, TrustedPeer},
    time::Duration,
};
use thor::{DummySyncNode, FragmentSender};

const LEADER1: &str = "LEADER1";
const LEADER2: &str = "LEADER2";
const LEADER3: &str = "LEADER3";
const LEADER4: &str = "LEADER4";

const CLIENT: &str = "CLIENT";
const SERVER: &str = "SERVER";
const SERVER_1: &str = "SERVER_1";
const SERVER_2: &str = "SERVER_2";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";

#[test]
// FIXME: ignored until we fix this issue
#[ignore]
pub fn max_connections() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER1))
                .with_node(Node::new(LEADER2).with_trusted_peer(LEADER1))
                .with_node(Node::new(LEADER3).with_trusted_peer(LEADER1))
                .with_node(Node::new(LEADER4).with_trusted_peer(LEADER1)),
        )
        .blockchain_config(
            Blockchain::default().with_leaders(vec![LEADER1, LEADER2, LEADER3, LEADER4]),
        )
        .build()
        .unwrap();

    let leader1 = controller
        .spawn(
            SpawnParams::new(LEADER1)
                .in_memory()
                .max_inbound_connections(2),
        )
        .unwrap();

    let _leader2 = controller
        .spawn(SpawnParams::new(LEADER2).in_memory())
        .unwrap();

    let _leader3 = controller
        .spawn(SpawnParams::new(LEADER3).in_memory())
        .unwrap();

    let _leader4 = controller
        .spawn(SpawnParams::new(LEADER4).in_memory())
        .unwrap();

    utils::wait(30);
    super::assert_connected_cnt(&leader1, 2, "leader1 should have only 2 nodes connected");
}

#[test]
pub fn node_trust_itself() {
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER))
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER)),
        )
        .blockchain_config(Blockchain::default().with_leader(SERVER))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(1_000_000)
                .delegated_to(CLIENT)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(1_000_000)
                .delegated_to(SERVER)
                .build(),
        )
        .build()
        .unwrap();

    let _server = network_controller
        .spawn(SpawnParams::new(SERVER).in_memory())
        .unwrap();

    let config = network_controller.node_config(CLIENT).unwrap().p2p;

    let peer = TrustedPeer {
        address: config.public_address,
        id: None,
    };
    network_controller
        .expect_spawn_failed(
            SpawnParams::new(CLIENT).trusted_peers(vec![peer]),
            "failed to retrieve the list of bootstrap peers from trusted peer",
        )
        .unwrap();
}

#[test]
/// Ensures intervals between gossip attempts respect the `gossip_interval` timing parameter
fn gossip_interval() {
    const SERVER_GOSSIP_INTERVAL_SECS: u64 = 3;
    const DEFAULT_GOSSIP_INTERVAL_SECS: u64 = 10;

    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER))
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER)),
        )
        .blockchain_config(Blockchain::default().with_leader(SERVER))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(1_000_000)
                .delegated_to(CLIENT)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(1_000_000)
                .delegated_to(SERVER)
                .build(),
        )
        .build()
        .unwrap();

    let server = network_controller
        .spawn(
            SpawnParams::new(SERVER)
                .gossip_interval(Duration::new(SERVER_GOSSIP_INTERVAL_SECS, 0))
                .log_level(LogLevel::TRACE),
        )
        .unwrap();

    let _client = network_controller
        .spawn(SpawnParams::new(CLIENT).in_memory())
        .unwrap();

    //check server gets the gossip sent by client every default gossip_interval
    let last_gossip = server
        .rest()
        .network_stats()
        .unwrap()
        .last()
        .unwrap()
        .last_gossip_received
        .unwrap();

    utils::wait(DEFAULT_GOSSIP_INTERVAL_SECS);

    let next_last_gossip = server
        .rest()
        .network_stats()
        .unwrap()
        .last()
        .unwrap()
        .last_gossip_received
        .unwrap();

    let client_gossip_interval = next_last_gossip
        .duration_since(last_gossip)
        .unwrap()
        .as_secs_f64()
        .round() as u64;

    assert_eq!(DEFAULT_GOSSIP_INTERVAL_SECS, client_gossip_interval);

    let log_timestamps: Vec<u64> = server
        .logger
        .get_lines_as_string()
        .into_iter()
        .filter(|s| s.contains("gossiping with peers"))
        .map(|t| parse_timestamp(&t))
        .collect();

    let mut prev = None;

    for log_timestamp in log_timestamps {
        match prev {
            None => prev = Some(log_timestamp),
            Some(p) => {
                assert!(log_timestamp - p >= SERVER_GOSSIP_INTERVAL_SECS);
                prev = Some(log_timestamp);
            }
        }
    }
}

#[cfg(feature = "soak-non-functional")]
#[test]
/// Ensures that consecutive network-stuck checks respect the `network_stuck_check` timing parameter
fn network_stuck_check() {
    const INTERVAL_SECS: u64 = 90;
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER))
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER)),
        )
        .blockchain_config(Blockchain::default().with_leader(SERVER))
        .build()
        .unwrap();

    let server = network_controller
        .spawn(SpawnParams::new(SERVER).in_memory())
        .unwrap();

    let client = network_controller
        .spawn(
            SpawnParams::new(CLIENT)
                .log_level(LogLevel::TRACE)
                .gossip_interval(Duration::new(5, 0))
                .network_stuck_check(Duration::new(INTERVAL_SECS, 0)),
        )
        .unwrap();

    server.stop();

    utils::wait(10 * INTERVAL_SECS);

    let log_timestamps: Vec<u64> = client
        .logger
        .get_lines_as_string()
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
pub fn topics_of_interest_influences_node_sync_ability() {
    const FAST_CLIENT: &str = "FAST_CLIENT";
    const SLOW_CLIENT: &str = "SLOW_CLIENT";

    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER))
                .with_node(Node::new(FAST_CLIENT).with_trusted_peer(SERVER))
                .with_node(Node::new(SLOW_CLIENT).with_trusted_peer(SERVER)),
        )
        .blockchain_config(Blockchain::default().with_leader(SERVER))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(1_000_000)
                .delegated_to(SERVER)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(1_000_000)
                .delegated_to(SERVER)
                .build(),
        )
        .build()
        .unwrap();

    let server = network_controller
        .spawn(SpawnParams::new(SERVER).in_memory())
        .unwrap();

    let fast_client = network_controller
        .spawn(
            SpawnParams::new(FAST_CLIENT)
                .in_memory()
                .topics_of_interest(TopicsOfInterest {
                    messages: "high".to_string(),
                    blocks: "high".to_string(),
                }),
        )
        .unwrap();
    let slow_client = network_controller
        .spawn(
            SpawnParams::new(SLOW_CLIENT)
                .in_memory()
                .topics_of_interest(TopicsOfInterest {
                    messages: "low".to_string(),
                    blocks: "low".to_string(),
                }),
        )
        .unwrap();

    let mut alice = network_controller.controlled_wallet(ALICE).unwrap();
    let mut bob = network_controller.controlled_wallet(BOB).unwrap();

    let fragment_sender: FragmentSender<DummySyncNode> =
        FragmentSender::from(&network_controller.settings().block0);
    fragment_sender
        .send_transactions_round_trip(40, &mut alice, &mut bob, &server, 100.into())
        .unwrap();

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
        fast_client_block_recv_cnt >= slow_client_block_recv_cnt,
        "node with high block topic of interest should have more recieved blocks fast:{} vs slow:{}",fast_client_block_recv_cnt,slow_client_block_recv_cnt
    );

    server.assert_no_errors_in_log();
    fast_client.assert_no_errors_in_log();
    slow_client.assert_no_errors_in_log();
}

#[test]
/// Ensures that a node will only attempt to bootstrap `max_boostrap_attempts` times
fn max_bootstrap_attempts() {
    const ATTEMPTS: usize = 3;

    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER))
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER)),
        )
        .blockchain_config(Blockchain::default().with_leader(SERVER))
        .build()
        .unwrap();

    let client = network_controller
        .spawn(
            SpawnParams::new(CLIENT)
                .max_bootstrap_attempts(ATTEMPTS)
                .log_level(LogLevel::TRACE),
        )
        .unwrap();

    utils::wait(5);

    assert_eq!(
        client
            .logger
            .get_lines_as_string()
            .into_iter()
            .filter(|l| l.contains("bootstrap attempt #"))
            .count(),
        ATTEMPTS
    );
}

pub fn parse_timestamp(log: &str) -> u64 {
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

#[test]
fn gossip_new_node_bootstrap() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(SERVER_1))
                .with_node(Node::new(SERVER_2).with_trusted_peer(SERVER_1))
                .with_node(Node::new(CLIENT).with_trusted_peer(SERVER_2)),
        )
        .blockchain_config(Blockchain::default().with_leaders(vec![SERVER_1]))
        .build()
        .unwrap();

    let server1 = controller
        .spawn(SpawnParams::new(SERVER_1).in_memory())
        .unwrap();

    let server2 = controller
        .spawn(SpawnParams::new(SERVER_2).in_memory())
        .unwrap();

    utils::wait(2);
    super::assert_are_in_network_view(&server1, vec![&server2], "Before second node bootstrap");
    super::assert_connected_cnt(&server1, 1, "Before second node bootstrap");

    let is_gossiping_with_one_node = server1
        .logger
        .get_lines_as_string()
        .iter()
        .any(|s| s.contains("received gossip on 1 nodes"));

    let is_gossiping_with_two_nodes = server1
        .logger
        .get_lines_as_string()
        .iter()
        .any(|s| s.contains("received gossip on 2 nodes"));

    assert!(is_gossiping_with_one_node, "Before second node bootstrap");
    assert!(!is_gossiping_with_two_nodes, "Before second node bootstrap");

    let client = controller.spawn(SpawnParams::new(CLIENT)).unwrap();

    utils::wait(2);
    super::assert_are_in_network_view(
        &server1,
        vec![&server2, &client],
        "After second node bootstrap",
    );
    super::assert_connected_cnt(&server1, 2, "After second node bootstrap");

    let is_gossiping_with_two_nodes = server1
        .logger
        .get_lines_as_string()
        .iter()
        .any(|s| s.contains("received gossip on 2 nodes"));

    assert!(is_gossiping_with_two_nodes, "After second node bootstrap");
}
