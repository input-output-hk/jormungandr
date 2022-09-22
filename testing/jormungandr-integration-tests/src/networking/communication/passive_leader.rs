use crate::networking::utils::wait;
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{Blockchain, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::{
    jormungandr::{LogLevel, MemPoolCheck},
    testing::{
        benchmark::{measure_and_log_sync_time, MeasurementReportInterval},
        SyncWaitParams,
    },
};
use jormungandr_lib::interfaces::Policy;
use std::time::Duration;
use thor::{FragmentSender, FragmentSenderSetup};

const PASSIVE: &str = "PASSIVE";
const LEADER: &str = "LEADER";
const LEADER_2: &str = "Leader2";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";
const CLARICE: &str = "CLARICE";

#[test]
pub fn two_nodes_communication() {
    let mut network_controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER))
                .with_node(Node::new(PASSIVE).with_trusted_peer(LEADER)),
        )
        .blockchain_config(Blockchain::default().with_leader(LEADER))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(1_000_000)
                .delegated_to(LEADER)
                .build(),
        )
        .wallet_template(WalletTemplateBuilder::new(BOB).with(1_000_000).build())
        .build()
        .unwrap();

    let leader = network_controller
        .spawn(SpawnParams::new(LEADER).in_memory())
        .unwrap();
    let passive = network_controller
        .spawn(SpawnParams::new(PASSIVE).in_memory().passive())
        .unwrap();

    let mut alice = network_controller.controlled_wallet(ALICE).unwrap();
    let mut bob = network_controller.controlled_wallet(BOB).unwrap();

    FragmentSender::from(&network_controller.settings().block0)
        .send_transactions_round_trip(5, &mut alice, &mut bob, &passive, 100.into())
        .expect("fragment send error");

    let fragment_ids: Vec<MemPoolCheck> = passive
        .rest()
        .fragment_logs()
        .unwrap()
        .iter()
        .map(|(id, _)| MemPoolCheck::new(*id))
        .collect();

    leader
        .correct_state_verifier()
        .fragment_logs()
        .assert_all_valid(&fragment_ids);
}

#[test]
pub fn transaction_to_passive() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER))
                .with_node(Node::new(PASSIVE).with_trusted_peer(LEADER)),
        )
        .blockchain_config(Blockchain::default().with_leader(LEADER))
        .wallet_template(WalletTemplateBuilder::new(ALICE).with(500_000_000).build())
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER)
                .build(),
        )
        .build()
        .unwrap();

    let leader = controller
        .spawn(SpawnParams::new(LEADER).in_memory())
        .unwrap();

    let passive = controller
        .spawn(SpawnParams::new(PASSIVE).in_memory().passive())
        .unwrap();

    let mut alice = controller.controlled_wallet(ALICE).unwrap();
    let mut bob = controller.controlled_wallet(BOB).unwrap();

    FragmentSender::from(&controller.settings().block0)
        .send_transactions_round_trip(10, &mut alice, &mut bob, &passive, 1_000.into())
        .unwrap();

    measure_and_log_sync_time(
        &[&passive, &leader],
        SyncWaitParams::two_nodes().into(),
        "transaction_to_passive_sync",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}

#[test]
pub fn leader_restart() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_2))
                .with_node(Node::new(LEADER).with_trusted_peer(LEADER_2))
                .with_node(
                    Node::new(PASSIVE)
                        .with_trusted_peer(LEADER)
                        .with_trusted_peer(LEADER_2),
                ),
        )
        .blockchain_config(Blockchain::default().with_leaders(vec![LEADER, LEADER_2]))
        .wallet_template(WalletTemplateBuilder::new(ALICE).with(500_000_000).build())
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(CLARICE)
                .with(2_000_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .build()
        .unwrap()
        .into_persistent();

    let qurantine_duration = 5;

    let policy = Policy {
        quarantine_duration: Some(Duration::new(qurantine_duration, 0).into()),
        quarantine_whitelist: None,
    };

    let leader_2 = controller
        .spawn(
            SpawnParams::new(LEADER_2)
                .in_memory()
                .policy(policy.clone()),
        )
        .unwrap();

    let mut leader = controller.spawn(SpawnParams::new(LEADER)).unwrap();

    let passive = controller
        .spawn(
            SpawnParams::new(PASSIVE)
                .policy(policy)
                .passive()
                .in_memory(),
        )
        .unwrap();

    let mut alice = controller.controlled_wallet(ALICE).unwrap();
    let mut bob = controller.controlled_wallet(BOB).unwrap();

    FragmentSender::from(&controller.settings().block0)
        .clone_with_setup(FragmentSenderSetup::resend_3_times())
        .send_transactions_round_trip(2, &mut alice, &mut bob, &passive, 1_000.into())
        .unwrap();

    leader.shutdown();
    leader
        .wait_for_shutdown(std::time::Duration::from_secs(10))
        .unwrap();
    FragmentSender::from(&controller.settings().block0)
        .clone_with_setup(FragmentSenderSetup::resend_3_times())
        .send_transactions_with_iteration_delay(
            4,
            &mut alice,
            &mut bob,
            &passive,
            1_000.into(),
            Duration::from_secs(3),
        )
        .unwrap();

    wait(qurantine_duration * 2);

    assert!(leader.ports_are_opened());
    let leader = controller
        .spawn(
            SpawnParams::new(LEADER)
                .verbose(true)
                .in_memory()
                .log_level(LogLevel::TRACE),
        )
        .unwrap();

    FragmentSender::from(&controller.settings().block0)
        .clone_with_setup(FragmentSenderSetup::resend_3_times())
        .send_transactions_round_trip(2, &mut alice, &mut bob, &passive, 1_000.into())
        .unwrap();

    measure_and_log_sync_time(
        &[&passive, &leader, &leader_2],
        SyncWaitParams::nodes_restart(2).into(),
        "leader_restart",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}

#[test]
pub fn passive_node_is_updated() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER))
                .with_node(Node::new(PASSIVE).with_trusted_peer(LEADER)),
        )
        .blockchain_config(Blockchain::default().with_leader(LEADER))
        .wallet_template(WalletTemplateBuilder::new(ALICE).with(500_000_000).build())
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER)
                .build(),
        )
        .build()
        .unwrap();

    let leader = controller
        .spawn(SpawnParams::new(LEADER).in_memory())
        .unwrap();

    let passive = controller
        .spawn(SpawnParams::new(PASSIVE).in_memory().passive())
        .unwrap();

    let mut alice = controller.controlled_wallet(ALICE).unwrap();
    let mut bob = controller.controlled_wallet(BOB).unwrap();

    FragmentSender::from(&controller.settings().block0)
        .send_transactions_round_trip(40, &mut alice, &mut bob, &leader, 1_000.into())
        .unwrap();

    measure_and_log_sync_time(
        &[&passive, &leader],
        SyncWaitParams::nodes_restart(2).into(),
        "passive_node_is_updated_sync",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}
