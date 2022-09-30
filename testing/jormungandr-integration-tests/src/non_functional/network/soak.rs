use crate::{
    networking::{p2p::connections::parse_timestamp, utils::wait},
    non_functional::network::*,
};
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{BlockchainBuilder, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::{
    jormungandr::LogLevel,
    testing::{ensure_nodes_are_in_sync, SyncWaitParams},
};
use std::time::{Duration, SystemTime};
use thor::{FragmentSender, FragmentVerifier};

const CORE_NODE: &str = "Core";
const RELAY_NODE_1: &str = "Relay1";
const RELAY_NODE_2: &str = "Relay2";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";
const CLARICE: &str = "CLARICE";
const DAVID: &str = "DAVID";
const EDGAR: &str = "EDGAR";
const FILIP: &str = "FILIP";
const GRACE: &str = "GRACE";

const LEADER_6: &str = "Leader6";
const LEADER_7: &str = "Leader7";

#[test]
pub fn relay_soak() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(CORE_NODE))
                .with_node(Node::new(RELAY_NODE_1).with_trusted_peer(CORE_NODE))
                .with_node(Node::new(RELAY_NODE_2).with_trusted_peer(CORE_NODE))
                .with_node(Node::new(LEADER_1).with_trusted_peer(RELAY_NODE_1))
                .with_node(Node::new(LEADER_2).with_trusted_peer(RELAY_NODE_1))
                .with_node(Node::new(LEADER_3).with_trusted_peer(RELAY_NODE_1))
                .with_node(Node::new(LEADER_4).with_trusted_peer(RELAY_NODE_2))
                .with_node(Node::new(LEADER_5).with_trusted_peer(RELAY_NODE_2))
                .with_node(Node::new(LEADER_6).with_trusted_peer(RELAY_NODE_2))
                .with_node(Node::new(LEADER_7).with_trusted_peer(RELAY_NODE_2)),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .delegated_to(LEADER_1)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_3)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_4)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_5)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_6)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_7)
                .build(),
        )
        .blockchain_config(
            BlockchainBuilder::default()
                .slots_per_epoch(60)
                .slot_duration(10)
                .build(),
        )
        .build()
        .unwrap();

    let _core = controller
        .spawn(SpawnParams::new(CORE_NODE).in_memory())
        .unwrap();

    let relay1 = controller
        .spawn(SpawnParams::new(RELAY_NODE_1).in_memory().passive())
        .unwrap();
    let relay2 = controller
        .spawn(SpawnParams::new(RELAY_NODE_2).in_memory().passive())
        .unwrap();

    let leader1 = controller
        .spawn(SpawnParams::new(LEADER_1).in_memory())
        .unwrap();
    let leader2 = controller
        .spawn(SpawnParams::new(LEADER_2).in_memory())
        .unwrap();
    let leader3 = controller
        .spawn(SpawnParams::new(LEADER_3).in_memory())
        .unwrap();
    let leader4 = controller
        .spawn(SpawnParams::new(LEADER_4).in_memory())
        .unwrap();
    let leader5 = controller
        .spawn(SpawnParams::new(LEADER_5).in_memory())
        .unwrap();
    let leader6 = controller
        .spawn(SpawnParams::new(LEADER_6).in_memory())
        .unwrap();
    let leader7 = controller
        .spawn(SpawnParams::new(LEADER_7).in_memory())
        .unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet2 = controller.controlled_wallet(BOB).unwrap();
    let mut wallet3 = controller.controlled_wallet(CLARICE).unwrap();
    let mut wallet4 = controller.controlled_wallet(DAVID).unwrap();
    let mut wallet5 = controller.controlled_wallet(EDGAR).unwrap();
    let mut wallet6 = controller.controlled_wallet(FILIP).unwrap();
    let mut wallet7 = controller.controlled_wallet(GRACE).unwrap();

    let now = SystemTime::now();

    let fragment_sender = FragmentSender::from(&controller.settings().block0);

    loop {
        let check1 = fragment_sender
            .send_transaction(&mut wallet1, &wallet2, &leader1, 1_000.into())
            .unwrap();
        let check2 = fragment_sender
            .send_transaction(&mut wallet2, &wallet1, &leader2, 1_000.into())
            .unwrap();
        let check3 = fragment_sender
            .send_transaction(&mut wallet3, &wallet4, &leader3, 1_000.into())
            .unwrap();
        let check4 = fragment_sender
            .send_transaction(&mut wallet4, &wallet3, &leader4, 1_000.into())
            .unwrap();
        let check5 = fragment_sender
            .send_transaction(&mut wallet5, &wallet6, &leader5, 1_000.into())
            .unwrap();
        let check6 = fragment_sender
            .send_transaction(&mut wallet6, &wallet1, &leader6, 1_000.into())
            .unwrap();
        let check7 = fragment_sender
            .send_transaction(&mut wallet7, &wallet6, &leader7, 1_000.into())
            .unwrap();

        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check1, &leader1)
            .unwrap();
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check2, &leader2)
            .unwrap();
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check3, &leader3)
            .unwrap();
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check4, &leader4)
            .unwrap();
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check5, &leader5)
            .unwrap();
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check6, &leader6)
            .unwrap();
        FragmentVerifier::wait_and_verify_is_in_block(Duration::from_secs(2), check7, &leader7)
            .unwrap();

        wallet1.confirm_transaction();
        wallet2.confirm_transaction();
        wallet3.confirm_transaction();
        wallet4.confirm_transaction();
        wallet5.confirm_transaction();
        wallet6.confirm_transaction();
        wallet7.confirm_transaction();

        // 48 hours
        if now.elapsed().unwrap().as_secs() > (900) {
            break;
        }
    }

    ensure_nodes_are_in_sync(
        SyncWaitParams::ZeroWait,
        &[
            &leader1, &leader2, &leader3, &leader4, &leader5, &leader6, &leader7, &relay1, &relay2,
        ],
    )
    .unwrap();
}

#[test]
/// Ensures that consecutive network-stuck checks respect the `network_stuck_check` timing parameter
fn network_stuck_check() {
    const INTERVAL_SECS: u64 = 90;
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(CORE_NODE))
                .with_node(Node::new(LEADER_1).with_trusted_peer(CORE_NODE)),
        )
        .build()
        .unwrap();

    let server = network_controller
        .spawn(SpawnParams::new(CORE_NODE).in_memory())
        .unwrap();

    let client = network_controller
        .spawn(
            SpawnParams::new(LEADER_1)
                .log_level(LogLevel::TRACE)
                .gossip_interval(jormungandr_lib::time::Duration::new(5, 0))
                .network_stuck_check(jormungandr_lib::time::Duration::new(INTERVAL_SECS, 0)),
        )
        .unwrap();

    server.stop();

    wait(10 * INTERVAL_SECS);

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
