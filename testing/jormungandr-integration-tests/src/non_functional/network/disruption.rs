use crate::{networking::utils, non_functional::network::*};
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::{
    jormungandr::{LeadershipMode, PersistenceMode},
    testing::{benchmark::MeasurementReportInterval, SyncWaitParams},
};
use thor::FragmentSender;
#[test]
pub fn passive_leader_disruption_no_overlap() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER))
                .with_node(Node::new(PASSIVE).with_trusted_peer(LEADER)),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER)
                .build(),
        )
        .build()
        .unwrap();

    let leader = controller.spawn(SpawnParams::new(LEADER)).unwrap();
    let passive = controller
        .spawn(SpawnParams::new(PASSIVE).passive())
        .unwrap();

    // 1. both nodes are up
    utils::wait(5);

    // 2. Only passive is down
    leader.shutdown();

    // 3. No node is up
    passive.shutdown();

    // 4. Only leader is up
    let leader = controller.spawn(SpawnParams::new(LEADER)).unwrap();
    utils::wait(5);

    // 5. No node is up
    leader.shutdown();

    //6. Both nodes are up
    let leader = controller.spawn(SpawnParams::new(LEADER)).unwrap();
    let passive = controller
        .spawn(SpawnParams::new(PASSIVE).passive())
        .unwrap();

    utils::measure_and_log_sync_time(
        &[&leader, &passive],
        SyncWaitParams::nodes_restart(5).into(),
        "passive_leader_disruption_no_overlap",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}

#[test]
pub fn passive_leader_disruption_overlap() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER))
                .with_node(Node::new(PASSIVE).with_trusted_peer(LEADER)),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER)
                .build(),
        )
        .build()
        .unwrap();

    let leader = controller.spawn(SpawnParams::new(LEADER)).unwrap();
    let passive = controller
        .spawn(SpawnParams::new(PASSIVE).passive())
        .unwrap();

    // 1. both nodes are up
    utils::wait(5);

    // 2. Only leader is up
    passive.shutdown();

    // Wait a bit so that the leader can indeed notice that passive is offline
    utils::wait(15);

    // 3. Both nodes are up
    let passive = controller
        .spawn(SpawnParams::new(PASSIVE).passive())
        .unwrap();

    utils::measure_and_log_sync_time(
        &[&leader, &passive],
        SyncWaitParams::nodes_restart(5).into(),
        "passive_leader_disruption_overlap",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}

#[test]
pub fn leader_leader_disruption_overlap() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_2))
                .with_node(Node::new(LEADER_1).with_trusted_peer(LEADER_2)),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .build()
        .unwrap();

    let leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();

    // 1. second node is up
    utils::wait(5);

    // 2. Both nodes are up
    let leader1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();
    utils::wait(5);

    // 3. second node is down
    leader2.shutdown();
    utils::wait(15);

    // 4. both nodes are up
    let leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();

    utils::measure_and_log_sync_time(
        &[&leader1, &leader2],
        SyncWaitParams::nodes_restart(5).into(),
        "leader_leader_disruption_overlap",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}

#[test]
pub fn leader_leader_disruption_no_overlap() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_2))
                .with_node(Node::new(LEADER_1).with_trusted_peer(LEADER_2)),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .build()
        .unwrap();

    let leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();
    let leader1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();
    // 1. Both nodes are up
    utils::wait(5);

    // 2. Only node 2 is up
    leader1.shutdown();

    // 3. No nodes are up
    leader2.shutdown();

    // 4.- 5. is disabled due to restriction that trusted peer is down
    // 6. Both nodes are up
    let leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();
    let leader1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();

    utils::measure_and_log_sync_time(
        &[&leader1, &leader2],
        SyncWaitParams::nodes_restart(5).into(),
        "leader_leader_disruption_no_overlap",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}

#[test]
pub fn point_to_point_disruption() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_2))
                .with_node(Node::new(LEADER_1).with_trusted_peer(LEADER_2))
                .with_node(Node::new(LEADER_3).with_trusted_peer(LEADER_2)),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .build()
        .unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet2 = controller.controlled_wallet(BOB).unwrap();

    let leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();
    let leader1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();
    let leader3 = controller.spawn(SpawnParams::new(LEADER_3)).unwrap();

    FragmentSender::from(&controller.settings().block0)
        .send_transactions_round_trip(40, &mut wallet1, &mut wallet2, &leader1, 1_000.into())
        .unwrap();

    leader2.shutdown();

    utils::measure_and_log_sync_time(
        &[&leader1, &leader3],
        SyncWaitParams::nodes_restart(5).into(),
        "point_to_point_disruption",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}

#[test]
pub fn point_to_point_disruption_overlap() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_2))
                .with_node(Node::new(LEADER_1).with_trusted_peer(LEADER_2))
                .with_node(Node::new(LEADER_3).with_trusted_peer(LEADER_2)),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .build()
        .unwrap();

    let leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();
    let mut leader1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();

    println!("1. 2 and 1 is up");
    utils::wait(5);

    println!("2. node 1 is down");
    leader1.shutdown();

    let mut leader3 = controller.spawn(SpawnParams::new(LEADER_3)).unwrap();

    println!("3. only Node 3 is up");
    leader2.shutdown();

    println!("4. 1 and 3 is up");
    leader1 = controller
        .spawn(
            SpawnParams::new(LEADER_1)
                .leadership_mode(LeadershipMode::Leader)
                .persistence_mode(PersistenceMode::Persistent)
                .bootstrap_from_peers(false)
                .skip_bootstrap(true),
        )
        .unwrap();

    println!("5. 2 and 3 is up");
    leader1.shutdown();
    let leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();

    println!("6. 1 and 2 is up");
    leader3.shutdown();

    let mut leader1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();

    println!("7. only Node 3 is up");
    leader3 = controller.spawn(SpawnParams::new(LEADER_3)).unwrap();
    leader1.shutdown();
    leader2.shutdown();

    // Wait a bit so that leader 3 will fail communications with leader 1 and 2 and put them
    // under quarantine.
    // Given prolonged time without contacts, leader 3 will try to contact again known nodes (after quarantine has elapsed)
    // even if it had not received any update in recent times, in this case leader 1 and 2.
    utils::wait(20);

    println!("8. 1 and 3 is up");
    leader1 = controller
        .spawn(
            SpawnParams::new(LEADER_1)
                .leadership_mode(LeadershipMode::Leader)
                .persistence_mode(PersistenceMode::Persistent)
                .bootstrap_from_peers(false)
                .skip_bootstrap(true),
        )
        .unwrap();

    println!("9. all nodes are up");
    let leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();

    utils::measure_and_log_sync_time(
        &[&leader1, &leader2, &leader3],
        SyncWaitParams::nodes_restart(5).into(),
        "point_to_point_disruption_overlap",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}

#[test]
pub fn custom_network_disruption() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_5))
                .with_node(Node::new(LEADER_1).with_trusted_peer(LEADER_3))
                .with_node(
                    Node::new(LEADER_2)
                        .with_trusted_peer(LEADER_3)
                        .with_trusted_peer(LEADER_5),
                )
                .with_node(Node::new(LEADER_3).with_trusted_peer(LEADER_5))
                .with_node(Node::new(LEADER_4).with_trusted_peer(LEADER_5))
                .with_node(Node::new(PASSIVE).with_trusted_peer(LEADER_5)),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_5)
                .build(),
        )
        .build()
        .unwrap();

    let leader5 = controller.spawn(SpawnParams::new(LEADER_5)).unwrap();

    let leader4 = controller.spawn(SpawnParams::new(LEADER_4)).unwrap();
    let leader3 = controller.spawn(SpawnParams::new(LEADER_3)).unwrap();
    let leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet3 = controller.controlled_wallet(BOB).unwrap();

    let fragment_sender = FragmentSender::from(&controller.settings().block0);

    fragment_sender
        .send_transactions_round_trip(2, &mut wallet1, &mut wallet3, &leader2, 1_000.into())
        .unwrap();

    let leader1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();

    fragment_sender
        .send_transactions_round_trip(2, &mut wallet1, &mut wallet3, &leader3, 1_000.into())
        .unwrap();

    leader2.shutdown();

    let passive = controller.spawn(SpawnParams::new(PASSIVE)).unwrap();

    fragment_sender
        .send_transactions_round_trip(2, &mut wallet1, &mut wallet3, &passive, 1_000.into())
        .unwrap();

    utils::measure_and_log_sync_time(
        &[&leader1, &leader3, &leader4, &leader5, &passive],
        SyncWaitParams::nodes_restart(5).into(),
        "custom_network_disruption",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}

#[test]
pub fn mesh_disruption() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_4))
                .with_node(Node::new(LEADER_1).with_trusted_peer(LEADER_4))
                .with_node(
                    Node::new(LEADER_2)
                        .with_trusted_peer(LEADER_1)
                        .with_trusted_peer(LEADER_4),
                )
                .with_node(
                    Node::new(LEADER_3)
                        .with_trusted_peer(LEADER_1)
                        .with_trusted_peer(LEADER_2),
                )
                .with_node(
                    Node::new(LEADER_5)
                        .with_trusted_peer(LEADER_2)
                        .with_trusted_peer(LEADER_1),
                ),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_5)
                .build(),
        )
        .build()
        .unwrap();

    let leader4 = controller.spawn(SpawnParams::new(LEADER_4)).unwrap();
    let leader1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();
    let mut leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();
    let mut leader5 = controller.spawn(SpawnParams::new(LEADER_5)).unwrap();
    let leader3 = controller.spawn(SpawnParams::new(LEADER_3)).unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet2 = controller.controlled_wallet(BOB).unwrap();

    let sender = FragmentSender::from(&controller.settings().block0);

    sender
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader1, 1_000.into())
        .unwrap();

    leader2.shutdown();
    leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();

    sender
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader1, 1_000.into())
        .unwrap();

    leader5.shutdown();
    leader5 = controller.spawn(SpawnParams::new(LEADER_5)).unwrap();

    sender
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader1, 1_000.into())
        .unwrap();

    utils::measure_and_log_sync_time(
        &[&leader1, &leader2, &leader3, &leader4, &leader5],
        SyncWaitParams::nodes_restart(5).into(),
        "mesh_disruption_sync",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}
