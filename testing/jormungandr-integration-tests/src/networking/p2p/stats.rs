use crate::networking::utils;
use chain_impl_mockchain::header::BlockDate;
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{Blockchain, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::jormungandr::JormungandrProcess;
use jormungandr_lib::interfaces::{NodeStats, Policy, SlotDuration};
use std::{fmt::Display, time::Duration};
use thor::FragmentSender;

const LEADER1: &str = "LEADER1";
const LEADER2: &str = "LEADER2";
const LEADER3: &str = "LEADER3";
const LEADER4: &str = "LEADER4";

const PASSIVE: &str = "PASSIVE";
const LEADER_CLIENT: &str = "LEADER_CLIENT";
const LEADER: &str = "LEADER";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";
const CLARICE: &str = "CLARICE";

#[test]
pub fn p2p_stats_test() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER1))
                .with_node(Node::new(LEADER2).with_trusted_peer(LEADER1))
                .with_node(Node::new(LEADER3).with_trusted_peer(LEADER1))
                .with_node(
                    Node::new(LEADER4)
                        .with_trusted_peer(LEADER2)
                        .with_trusted_peer(LEADER3),
                ),
        )
        .blockchain_config(Blockchain::default().with_leaders(vec![LEADER1, LEADER2, LEADER3]))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_000_000_000)
                .delegated_to(LEADER1)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER2)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(CLARICE)
                .with(2_000_000_000)
                .delegated_to(LEADER3)
                .build(),
        )
        .build()
        .unwrap();

    let policy = Policy {
        quarantine_duration: Some(Duration::new(120, 0).into()),
        quarantine_whitelist: None,
    };

    let leader1 = controller
        .spawn(SpawnParams::new(LEADER1).in_memory().policy(policy.clone()))
        .unwrap();

    super::assert_node_stats(&leader1, 0, 0, 0, "no peers for leader1");
    let info_before = "no peers for leader 1";
    assert!(
        leader1.rest().network_stats().unwrap().is_empty(),
        "{} network_stats",
        info_before,
    );
    assert!(
        leader1.rest().p2p_quarantined().unwrap().is_empty(),
        "{} p2p_quarantined",
        info_before,
    );
    assert!(
        leader1.rest().p2p_non_public().unwrap().is_empty(),
        "{} p2p_non_public",
        info_before,
    );
    assert!(
        leader1.rest().p2p_available().unwrap().is_empty(),
        "{} p2p_available",
        info_before,
    );
    assert!(
        leader1.rest().p2p_view().unwrap().is_empty(),
        "{} p2p_view",
        info_before,
    );

    let leader2 = controller
        .spawn(
            SpawnParams::new(LEADER2)
                .in_memory()
                .no_listen_address()
                .policy(policy.clone()),
        )
        .unwrap();

    utils::wait(20);
    super::assert_node_stats(&leader1, 1, 0, 1, "bootstrapped leader1");
    super::assert_node_stats(&leader2, 1, 0, 1, "bootstrapped leader2");

    let leader3 = controller
        .spawn(
            SpawnParams::new(LEADER3)
                .in_memory()
                .no_listen_address()
                .policy(policy.clone()),
        )
        .unwrap();

    utils::wait(20);
    super::assert_node_stats(&leader1, 2, 0, 2, "leader1: leader3 node is up");
    super::assert_node_stats(&leader2, 2, 0, 2, "leader2: leader3 node is up");
    super::assert_node_stats(&leader3, 2, 0, 2, "leader3: leader3 node is up");

    let leader4 = controller
        .spawn(
            SpawnParams::new(LEADER4)
                .in_memory()
                .no_listen_address()
                .policy(policy),
        )
        .unwrap();

    utils::wait(20);
    super::assert_node_stats(&leader1, 3, 0, 3, "leader1: leader4 node is up");
    super::assert_node_stats(&leader2, 3, 0, 3, "leader2: leader4 node is up");
    super::assert_node_stats(&leader3, 3, 0, 3, "leader3: leader4 node is up");
    super::assert_node_stats(&leader3, 3, 0, 3, "leader4: leader4 node is up");

    leader2.shutdown();
    utils::wait(20);
    //TODO try to determine why quarantine counter id not bumped up
    super::assert_node_stats(&leader1, 3, 0, 3, "leader1: leader 2 is down");
    super::assert_node_stats(&leader3, 3, 0, 3, "leader3: leader 2 is down");
    super::assert_node_stats(&leader4, 3, 0, 3, "leader4: leader 2 is down")
}

// build a blockchain with a longer slot duration than default
// to avoid spurious failures as described in
// https://github.com/input-output-hk/jormungandr/issues/3183.
// It is a macro because the builder is returned by reference.
macro_rules! build_network {
    () => {{
        NetworkBuilder::default().blockchain_config(
            Blockchain::default().with_slot_duration(SlotDuration::new(5).unwrap()),
        )
    }};
}

#[test]
pub fn passive_node_last_block_info() {
    let mut network_controller = build_network!()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER))
                .with_node(Node::new(PASSIVE).with_trusted_peer(LEADER)),
        )
        .blockchain_config(Blockchain::default().with_leader(LEADER))
        .wallet_template(
            WalletTemplateBuilder::new("alice")
                .with(1_000_000)
                .delegated_to(LEADER)
                .build(),
        )
        .wallet_template(WalletTemplateBuilder::new("bob").with(1_000_000).build())
        .build()
        .unwrap();

    let leader = network_controller
        .spawn(SpawnParams::new(LEADER).in_memory())
        .unwrap();
    let passive = network_controller
        .spawn(SpawnParams::new(PASSIVE).in_memory().passive())
        .unwrap();

    let mut alice = network_controller.controlled_wallet("alice").unwrap();
    let mut bob = network_controller.controlled_wallet("bob").unwrap();

    let stats_before = passive
        .rest()
        .stats()
        .expect("cannot get stats at beginning")
        .stats
        .expect("empty stats");

    let fragment_sender = FragmentSender::new(
        leader.genesis_block_hash(),
        leader.fees(),
        BlockDate::first().next_epoch().into(),
        Default::default(),
    );

    fragment_sender
        .send_transactions_round_trip(5, &mut alice, &mut bob, &leader, 100.into())
        .expect("fragment send error");

    assert_last_stats_are_updated(stats_before, &passive);
}

#[test]
pub fn leader_node_last_block_info() {
    let mut network_controller = build_network!()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER))
                .with_node(Node::new(LEADER_CLIENT).with_trusted_peer(LEADER)),
        )
        .wallet_template(
            WalletTemplateBuilder::new("alice")
                .with(1_000_000)
                .delegated_to(LEADER)
                .build(),
        )
        .wallet_template(WalletTemplateBuilder::new("bob").with(1_000_000).build())
        .blockchain_config(Blockchain::default().with_leader(LEADER))
        .build()
        .unwrap();

    let leader = network_controller
        .spawn(SpawnParams::new(LEADER).in_memory())
        .unwrap();
    let leader_client = network_controller
        .spawn(SpawnParams::new(LEADER_CLIENT).in_memory())
        .unwrap();

    let mut alice = network_controller.controlled_wallet("alice").unwrap();
    let mut bob = network_controller.controlled_wallet("bob").unwrap();

    let stats_before = leader_client
        .rest()
        .stats()
        .expect("cannot get stats at beginning")
        .stats
        .expect("empty stats");

    let fragment_sender = FragmentSender::new(
        leader.genesis_block_hash(),
        leader.fees(),
        BlockDate::first().next_epoch().into(),
        Default::default(),
    );

    fragment_sender
        .send_transactions_round_trip(5, &mut alice, &mut bob, &leader, 100.into())
        .expect("fragment send error");

    assert_last_stats_are_updated(stats_before, &leader_client);
}

fn assert_last_stats_are_updated(stats_before: NodeStats, node: &JormungandrProcess) {
    let stats_after = node
        .rest()
        .stats()
        .expect("cannot get stats at end")
        .stats
        .expect("empty stats");

    stats_element_is_different(
        stats_before.last_block_content_size,
        stats_after.last_block_content_size,
        "last block content size",
    );

    let before_last_block_date: BlockDate = stats_before.last_block_date.unwrap().parse().unwrap();
    let after_last_block_date: BlockDate = stats_after.last_block_date.unwrap().parse().unwrap();

    stats_element_is_greater(
        before_last_block_date,
        after_last_block_date,
        "last block date",
    );
    stats_element_is_different(
        stats_before.last_block_fees,
        stats_after.last_block_fees,
        "last block fees size",
    );

    stats_element_is_different(
        stats_before.last_block_hash.unwrap(),
        stats_after.last_block_hash.unwrap(),
        "last block hash",
    );
    stats_element_is_different(
        stats_before.last_block_sum,
        stats_after.last_block_sum,
        "last block sum",
    );
    stats_element_is_greater(
        stats_before.last_block_time.unwrap(),
        stats_after.last_block_time.unwrap(),
        "last block time",
    );
    stats_element_is_different(
        stats_before.last_block_tx,
        stats_after.last_block_tx,
        "last block tx",
    );
}

fn stats_element_is_greater<T>(before_value: T, after_value: T, info: &str)
where
    T: Display + PartialOrd,
{
    assert!(
        before_value < after_value,
        "{} should to be updated. {} vs {}",
        info,
        before_value,
        after_value,
    );
}

fn stats_element_is_different<T>(before_value: T, after_value: T, info: &str)
where
    T: Display + PartialOrd,
{
    assert!(
        before_value != after_value,
        "{} should to be updated. {} vs {}",
        info,
        before_value,
        after_value,
    );
}
