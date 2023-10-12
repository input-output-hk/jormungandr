use crate::networking::utils;
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{
        BlockchainBuilder, BlockchainConfiguration, NodeConfig, SpawnParams, WalletTemplateBuilder,
    },
};
use jormungandr_automation::{
    jormungandr::{explorer::configuration::ExplorerParams, LogLevel},
    testing::{ensure_nodes_are_in_sync, SyncWaitParams},
};

use jormungandr_lib::{
    interfaces::{Policy, PreferredListConfig, SlotDuration, TrustedPeer},
    time::{Duration, SystemTime},
};
use multiaddr::Multiaddr;
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6},
    path::PathBuf,
};
use thor::{FragmentSender, FragmentVerifier};

const GATEWAY: &str = "GATEWAY";

const PUBLIC_NODE: &str = "PUBLIC";
const INTERNAL_NODE: &str = "INTERNAL";
const INTERNAL_NODE_2: &str = "INTERNAL_2";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";

#[ignore]
#[test]
fn public_gossip_rejection() {
    const SERVER_GOSSIP_INTERVAL_SECS: u64 = 10;

    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(INTERNAL_NODE))
                .with_node(Node::new(GATEWAY).with_trusted_peer(INTERNAL_NODE))
                .with_node(Node::new(PUBLIC_NODE).with_trusted_peer(GATEWAY)),
        )
        .blockchain_config(BlockchainConfiguration::default().with_leader(GATEWAY))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(1_000_000)
                .delegated_to(INTERNAL_NODE)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(1_000_000)
                .delegated_to(GATEWAY)
                .build(),
        )
        .build()
        .unwrap();

    // spin up node within the intranet
    // gossip from public node should be dropped

    let _client_internal = network_controller
        .spawn(
            SpawnParams::new(INTERNAL_NODE)
                .gossip_interval(Duration::new(5, 0))
                .allow_private_addresses(false),
        )
        .unwrap();

    // node from internal network exposed to public
    let _gateway = network_controller
        .spawn(
            SpawnParams::new(GATEWAY)
                .gossip_interval(Duration::new(SERVER_GOSSIP_INTERVAL_SECS, 0))
                .allow_private_addresses(true)
                .log_level(LogLevel::TRACE),
        )
        .unwrap();

    // simulate node in the wild
    let address: Multiaddr = "/ip4/80.9.12.3/tcp/0".parse().unwrap();

    let _client_public = network_controller
        .spawn(
            SpawnParams::new(PUBLIC_NODE)
                .gossip_interval(Duration::new(5, 0))
                .public_address(address)
                .allow_private_addresses(true),
        )
        .unwrap();

    utils::wait(20);

    let mut gossip_dropped = false;
    // internal node should drop gossip from public node
    for i in _client_internal.logger.get_lines_as_string().into_iter() {
        if i.contains("nodes dropped from gossip") && i.contains("80.9.12.3") {
            gossip_dropped = true
        }
    }

    assert!(gossip_dropped);
}

#[ignore]
#[test]
pub fn test_public_node_cannot_publish() {
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(INTERNAL_NODE))
                .with_node(Node::new(INTERNAL_NODE_2).with_trusted_peer(INTERNAL_NODE))
                .with_node(Node::new(GATEWAY).with_trusted_peer(INTERNAL_NODE))
                .with_node(Node::new(PUBLIC_NODE).with_trusted_peer(GATEWAY)),
        )
        .blockchain_config(BlockchainConfiguration::default().with_leader(INTERNAL_NODE))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(1_000_000)
                .delegated_to(INTERNAL_NODE)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(1_000_000)
                .delegated_to(INTERNAL_NODE_2)
                .build(),
        )
        .build()
        .unwrap();

    //
    //
    // Get addresses of nodes for whitelisting
    let internal_node_addr = network_controller
        .node_config(INTERNAL_NODE)
        .unwrap()
        .p2p
        .get_listen_addr()
        .unwrap();

    let internal_node_2_addr = network_controller
        .node_config(INTERNAL_NODE_2)
        .unwrap()
        .p2p
        .get_listen_addr()
        .unwrap();

    let gateway_addr = network_controller
        .node_config(GATEWAY)
        .unwrap()
        .p2p
        .get_listen_addr()
        .unwrap();

    let whitelist = vec![internal_node_addr, internal_node_2_addr, gateway_addr];

    println!("whitelist {:?}", whitelist);

    //
    //
    // add whitelists to nodes config
    let mut internal_node_config = network_controller.node_config(INTERNAL_NODE).unwrap();

    internal_node_config.p2p.whitelist = Some(whitelist.clone());

    let mut internal_node_2_config = network_controller.node_config(INTERNAL_NODE_2).unwrap();

    internal_node_2_config.p2p.whitelist = Some(whitelist.clone());

    let mut gateway_node_config = network_controller.node_config(GATEWAY).unwrap();

    gateway_node_config.p2p.whitelist = Some(whitelist.clone());

    let mut public_node_config = network_controller.node_config(PUBLIC_NODE).unwrap();

    public_node_config.p2p.whitelist = Some(whitelist.clone());

    //
    //
    // spin up internal nodes
    let params = SpawnParams::new(INTERNAL_NODE)
        .gossip_interval(Duration::new(1, 0))
        .allow_private_addresses(false)
        .whitelist(whitelist.clone());

    params.override_settings(&mut internal_node_config);

    let _client_internal = network_controller.spawn(params).unwrap();

    let params = SpawnParams::new(INTERNAL_NODE_2)
        .gossip_interval(Duration::new(1, 0))
        .allow_private_addresses(false)
        .whitelist(whitelist.clone());

    params.override_settings(&mut internal_node_2_config);

    let _client_internal_2 = network_controller.spawn(params).unwrap();

    //
    //
    // node from internal network exposed to public

    let params = SpawnParams::new(GATEWAY)
        .gossip_interval(Duration::new(1, 0))
        .allow_private_addresses(true)
        .whitelist(whitelist.clone());

    params.override_settings(&mut gateway_node_config);

    let _gateway = network_controller.spawn(params).unwrap();

    //
    //
    // simulate node in the wild
    let address: Multiaddr = "/ip4/80.9.12.3/tcp/0".parse().unwrap();

    let params = SpawnParams::new(PUBLIC_NODE)
        .gossip_interval(Duration::new(1, 0))
        .allow_private_addresses(true)
        .public_address(address)
        .whitelist(whitelist.clone());

    params.override_settings(&mut public_node_config);

    let _client_public = network_controller.spawn(params).unwrap();

    //
    //
    // public node sends fragments to network
    // it should fail as they public node is not whitelisted
    let mut alice = network_controller.controlled_wallet(ALICE).unwrap();
    let mut bob = network_controller.controlled_wallet(BOB).unwrap();

    let fragment_sender = FragmentSender::from(&network_controller.settings().block0);

    match fragment_sender.send_transactions_round_trip(
        5,
        &mut alice,
        &mut bob,
        &_client_public,
        100.into(),
    ) {
        Ok(_) => panic!("public node is not whitelisted, fragments should not send!"),
        Err(err) => assert_eq!(
            "Too many attempts failed (1) while trying to send fragment to node: ".to_string(),
            err.to_string()
        ),
    };
}

#[ignore]
#[test]
pub fn test_public_node_synced_with_internal() {
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(INTERNAL_NODE))
                .with_node(Node::new(INTERNAL_NODE_2).with_trusted_peer(INTERNAL_NODE))
                .with_node(Node::new(GATEWAY).with_trusted_peer(INTERNAL_NODE))
                .with_node(Node::new(PUBLIC_NODE).with_trusted_peer(GATEWAY)),
        )
        .blockchain_config(BlockchainConfiguration::default().with_leader(INTERNAL_NODE))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(1_000_000)
                .delegated_to(INTERNAL_NODE)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(1_000_000)
                .delegated_to(INTERNAL_NODE_2)
                .build(),
        )
        .build()
        .unwrap();

    //
    //
    // Get addresses of nodes for whitelisting
    let internal_node_addr = network_controller
        .node_config(INTERNAL_NODE)
        .unwrap()
        .p2p
        .get_listen_addr()
        .unwrap();

    let internal_node_2_addr = network_controller
        .node_config(INTERNAL_NODE_2)
        .unwrap()
        .p2p
        .get_listen_addr()
        .unwrap();

    let gateway_addr = network_controller
        .node_config(GATEWAY)
        .unwrap()
        .p2p
        .get_listen_addr()
        .unwrap();

    let whitelist = vec![internal_node_addr, internal_node_2_addr, gateway_addr];

    println!("whitelist {:?}", whitelist);

    //
    //
    // add whitelists to nodes config
    let mut internal_node_config = network_controller.node_config(INTERNAL_NODE).unwrap();

    internal_node_config.p2p.whitelist = Some(whitelist.clone());

    let mut internal_node_2_config = network_controller.node_config(INTERNAL_NODE_2).unwrap();

    internal_node_2_config.p2p.whitelist = Some(whitelist.clone());

    let mut gateway_node_config = network_controller.node_config(GATEWAY).unwrap();

    gateway_node_config.p2p.whitelist = Some(whitelist.clone());

    let mut public_node_config = network_controller.node_config(PUBLIC_NODE).unwrap();

    public_node_config.p2p.whitelist = Some(whitelist.clone());

    //
    //
    // spin up internal nodes
    let params = SpawnParams::new(INTERNAL_NODE)
        .gossip_interval(Duration::new(1, 0))
        .allow_private_addresses(false)
        .whitelist(whitelist.clone());

    params.override_settings(&mut internal_node_config);

    let _client_internal = network_controller.spawn(params).unwrap();

    let params = SpawnParams::new(INTERNAL_NODE_2)
        .gossip_interval(Duration::new(1, 0))
        .allow_private_addresses(false)
        .whitelist(whitelist.clone());

    params.override_settings(&mut internal_node_2_config);

    let _client_internal_2 = network_controller.spawn(params).unwrap();

    //
    //
    // node from internal network exposed to public

    let params = SpawnParams::new(GATEWAY)
        .gossip_interval(Duration::new(1, 0))
        .allow_private_addresses(true)
        .whitelist(whitelist.clone());

    params.override_settings(&mut gateway_node_config);

    let _gateway = network_controller.spawn(params).unwrap();

    //
    //
    // simulate node in the wild
    let address: Multiaddr = "/ip4/80.9.12.3/tcp/0".parse().unwrap();

    let params = SpawnParams::new(PUBLIC_NODE)
        .gossip_interval(Duration::new(1, 0))
        .allow_private_addresses(true)
        .public_address(address)
        .whitelist(whitelist.clone());

    params.override_settings(&mut public_node_config);

    let _client_public = network_controller.spawn(params).unwrap();

    //
    //
    // internal node sends fragments to network
    // fragments should be propagated to the publish node (which can consume but not publish)
    let mut alice = network_controller.controlled_wallet(ALICE).unwrap();
    let mut bob = network_controller.controlled_wallet(BOB).unwrap();

    let fragment_sender = FragmentSender::from(&network_controller.settings().block0);

    match fragment_sender.send_transactions_round_trip(
        5,
        &mut alice,
        &mut bob,
        &_client_internal,
        100.into(),
    ) {
        Ok(_) => println!("fragments sent"),
        Err(err) => panic!("{}", err),
    };

    utils::wait(10);

    //
    //
    // account states should be the same

    let public_state_a = _client_internal_2
        .rest()
        .account_state(&alice.account_id())
        .unwrap();

    let public_state_b = _client_public
        .rest()
        .account_state(&bob.account_id())
        .unwrap();

    let internal_state_a = _client_internal_2
        .rest()
        .account_state(&alice.account_id())
        .unwrap();

    let internal_state_b = _client_public
        .rest()
        .account_state(&bob.account_id())
        .unwrap();

    assert_eq!(public_state_a, internal_state_a);
    assert_eq!(public_state_b, internal_state_b);

    // based on this test; nodes will never be fully synced as gossip from the public node is dropped by internal nodes
    // which do not allow private addresses
    ensure_nodes_are_in_sync(
        SyncWaitParams::ZeroWait,
        &[&_client_internal_2, &_client_public],
    )
    .unwrap();
}
