use crate::common::{
    jormungandr::JormungandrProcess,
    network::{NetworkBuilder, WalletTemplateBuilder},
};
use chain_impl_mockchain::{chaintypes::ConsensusVersion, milli::Milli};
use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, KesUpdateSpeed, NumberOfSlotsPerEpoch, SlotDuration,
};
use jormungandr_testing_utils::testing::{
    fragments::BlockDateGenerator, network_builder::Blockchain,
};
use std::{cmp::PartialOrd, fmt::Display};

use chain_impl_mockchain::block::BlockDate;
use jormungandr_lib::interfaces::NodeStats;
use jormungandr_testing_utils::testing::FragmentSender;

const PASSIVE: &str = "PASSIVE";
const LEADER_CLIENT: &str = "LEADER_CLIENT";
const LEADER: &str = "LEADER";

// build a blockchain with a longer slot duration than default
// to avoid spurious failures as described in
// https://github.com/input-output-hk/jormungandr/issues/3183.
// It is a macro because the builder is returned by reference.
macro_rules! build_network {
    () => {
        NetworkBuilder::default().blockchain_config(Blockchain::new(
            ConsensusVersion::GenesisPraos,
            NumberOfSlotsPerEpoch::new(60).expect("valid number of slots per epoch"),
            SlotDuration::new(5).expect("valid slot duration in seconds"),
            KesUpdateSpeed::new(46800).expect("valid kes update speed in seconds"),
            ActiveSlotCoefficient::new(Milli::from_millis(999))
                .expect("active slot coefficient in millis"),
        ))
    };
}

#[test]
pub fn passive_node_last_block_info() {
    let mut network_controller = build_network!()
        .single_trust_direction(PASSIVE, LEADER)
        .initials(vec![
            WalletTemplateBuilder::new("alice")
                .with(1_000_000)
                .delegated_to(LEADER),
            WalletTemplateBuilder::new("bob").with(1_000_000),
        ])
        .build()
        .unwrap();

    let leader = network_controller.spawn_and_wait(LEADER);
    let passive = network_controller.spawn_as_passive_and_wait(PASSIVE);

    let mut alice = network_controller.wallet("alice").unwrap();
    let mut bob = network_controller.wallet("bob").unwrap();

    let stats_before = passive
        .rest()
        .stats()
        .expect("cannot get stats at beginning")
        .stats
        .expect("empty stats");

    let fragment_sender = FragmentSender::new(
        leader.genesis_block_hash(),
        leader.fees(),
        BlockDateGenerator::Fixed(BlockDate::first().next_epoch()),
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
        .single_trust_direction(LEADER_CLIENT, LEADER)
        .initials(vec![
            WalletTemplateBuilder::new("alice")
                .with(1_000_000)
                .delegated_to(LEADER),
            WalletTemplateBuilder::new("bob").with(1_000_000),
        ])
        .build()
        .unwrap();

    let leader = network_controller.spawn_and_wait(LEADER);
    let leader_client = network_controller.spawn_and_wait(LEADER_CLIENT);

    let mut alice = network_controller.wallet("alice").unwrap();
    let mut bob = network_controller.wallet("bob").unwrap();

    let stats_before = leader_client
        .rest()
        .stats()
        .expect("cannot get stats at beginning")
        .stats
        .expect("empty stats");

    let fragment_sender = FragmentSender::new(
        leader.genesis_block_hash(),
        leader.fees(),
        BlockDateGenerator::Fixed(BlockDate::first().next_epoch()),
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
