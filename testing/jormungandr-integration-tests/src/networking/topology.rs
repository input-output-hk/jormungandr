use crate::networking::utils;
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{Blockchain, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::testing::{benchmark::MeasurementReportInterval, SyncWaitParams};
use thor::{FragmentSender, FragmentSenderSetup};

const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";
const LEADER_3: &str = "Leader3";
const LEADER_4: &str = "Leader4";
const LEADER_5: &str = "Leader5";
const LEADER_6: &str = "Leader6";
const LEADER_7: &str = "Leader7";

const CORE_NODE: &str = "Core";
const RELAY_NODE_1: &str = "Relay1";
const RELAY_NODE_2: &str = "Relay2";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";

#[test]
pub fn fully_connected() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_3))
                .with_node(Node::new(LEADER_1).with_trusted_peer(LEADER_3))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1))
                .with_node(
                    Node::new(LEADER_4)
                        .with_trusted_peer(LEADER_2)
                        .with_trusted_peer(LEADER_3),
                ),
        )
        .blockchain_config(Blockchain::default().with_leaders(vec![LEADER_1, LEADER_2, LEADER_3]))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_000_000_000)
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

    let leader3 = controller
        .spawn(SpawnParams::new(LEADER_3).in_memory())
        .unwrap();

    let leader1 = controller
        .spawn(SpawnParams::new(LEADER_1).in_memory())
        .unwrap();

    let leader2 = controller
        .spawn(SpawnParams::new(LEADER_2).in_memory())
        .unwrap();

    let leader4 = controller
        .spawn(SpawnParams::new(LEADER_4).in_memory())
        .unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet2 = controller.controlled_wallet(BOB).unwrap();

    let fragment_sender = FragmentSender::from(&controller.settings().block0);

    fragment_sender
        .send_transactions_round_trip(10, &mut wallet1, &mut wallet2, &leader1, 1_000.into())
        .unwrap();

    let leaders = [&leader1, &leader2, &leader3, &leader4];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(4, 2).into(),
        "fully_connected_sync",
        MeasurementReportInterval::Standard,
    )
    .unwrap();

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(4, 2).into(),
        "fully_connected_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    );
}

#[test]
pub fn star() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_5))
                .with_node(Node::new(LEADER_1).with_trusted_peer(LEADER_5))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_5))
                .with_node(Node::new(LEADER_3).with_trusted_peer(LEADER_5))
                .with_node(Node::new(LEADER_4).with_trusted_peer(LEADER_5)),
        )
        .blockchain_config(
            Blockchain::default()
                .with_leaders(vec![LEADER_1, LEADER_2, LEADER_3, LEADER_4, LEADER_5]),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_000_000_000)
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

    let leader5 = controller
        .spawn(SpawnParams::new(LEADER_5).in_memory())
        .unwrap();
    let leader4 = controller
        .spawn(SpawnParams::new(LEADER_4).in_memory())
        .unwrap();
    let leader3 = controller
        .spawn(SpawnParams::new(LEADER_3).in_memory())
        .unwrap();
    let leader2 = controller
        .spawn(SpawnParams::new(LEADER_2).in_memory())
        .unwrap();
    let leader1 = controller
        .spawn(SpawnParams::new(LEADER_1).in_memory())
        .unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet2 = controller.controlled_wallet(BOB).unwrap();

    FragmentSender::from(&controller.settings().block0)
        .send_transactions_round_trip(40, &mut wallet1, &mut wallet2, &leader1, 1_000.into())
        .unwrap();

    let leaders = [&leader1, &leader2, &leader3, &leader4, &leader5];
    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "star_sync",
        MeasurementReportInterval::Standard,
    )
    .unwrap();

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "star_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    );
}

#[test]
pub fn mesh() {
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
                        .with_trusted_peer(LEADER_1)
                        .with_trusted_peer(LEADER_2),
                ),
        )
        .blockchain_config(
            Blockchain::default().with_leaders(vec![LEADER_1, LEADER_2, LEADER_3, LEADER_4]),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_000_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_3)
                .build(),
        )
        .build()
        .unwrap();

    let leader4 = controller
        .spawn(SpawnParams::new(LEADER_4).in_memory())
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
    let leader5 = controller
        .spawn(SpawnParams::new(LEADER_5).in_memory())
        .unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet2 = controller.controlled_wallet(BOB).unwrap();

    FragmentSender::from(&controller.settings().block0)
        .send_transactions_round_trip(4, &mut wallet1, &mut wallet2, &leader1, 1_000.into())
        .unwrap();

    let leaders = [&leader1, &leader2, &leader3, &leader4, &leader5];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "mesh_sync",
        MeasurementReportInterval::Standard,
    )
    .unwrap();

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(5, 3).into(),
        "mesh_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    );
}

#[test]
pub fn point_to_point() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_4))
                .with_node(Node::new(LEADER_3).with_trusted_peer(LEADER_4))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_3))
                .with_node(Node::new(LEADER_1).with_trusted_peer(LEADER_2)),
        )
        .blockchain_config(
            Blockchain::default().with_leaders(vec![LEADER_1, LEADER_2, LEADER_3, LEADER_4]),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_000_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_1)
                .build(),
        )
        .build()
        .unwrap();

    let leader4 = controller
        .spawn(SpawnParams::new(LEADER_4).in_memory())
        .unwrap();
    let leader3 = controller
        .spawn(SpawnParams::new(LEADER_3).in_memory())
        .unwrap();
    let leader2 = controller
        .spawn(SpawnParams::new(LEADER_2).in_memory())
        .unwrap();
    let leader1 = controller
        .spawn(SpawnParams::new(LEADER_1).in_memory())
        .unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet2 = controller.controlled_wallet(BOB).unwrap();

    FragmentSender::from(&controller.settings().block0)
        .send_transactions_round_trip(5, &mut wallet1, &mut wallet2, &leader1, 1_000.into())
        .unwrap();

    let leaders = [&leader1, &leader2, &leader3, &leader4];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(4, 4).into(),
        "point_to_point_sync",
        MeasurementReportInterval::Standard,
    )
    .unwrap();

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(4, 4).into(),
        "point_to_point_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    );
}

#[test]
pub fn point_to_point_on_file_storage() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_4))
                .with_node(Node::new(LEADER_3).with_trusted_peer(LEADER_4))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_3))
                .with_node(Node::new(LEADER_1).with_trusted_peer(LEADER_2)),
        )
        .blockchain_config(
            Blockchain::default().with_leaders(vec![LEADER_1, LEADER_2, LEADER_3, LEADER_4]),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_000_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_1)
                .build(),
        )
        .build()
        .unwrap();

    let leader4 = controller.spawn(SpawnParams::new(LEADER_4)).unwrap();
    let leader3 = controller.spawn(SpawnParams::new(LEADER_3)).unwrap();
    let leader2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();
    let leader1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();

    let mut wallet1 = controller.controlled_wallet(ALICE).unwrap();
    let mut wallet2 = controller.controlled_wallet(BOB).unwrap();

    FragmentSender::from(&controller.settings().block0)
        .send_transactions_round_trip(40, &mut wallet1, &mut wallet2, &leader1, 1_000.into())
        .unwrap();

    let leaders = [&leader1, &leader2, &leader3, &leader4];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(4, 4).into(),
        "point_to_point_on_file_storage_sync",
        MeasurementReportInterval::Standard,
    )
    .unwrap();

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(4, 4).into(),
        "point_to_point_on_file_storage_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    );
}

#[test]
pub fn tree() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_1))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1))
                .with_node(Node::new(LEADER_3).with_trusted_peer(LEADER_1))
                .with_node(Node::new(LEADER_4).with_trusted_peer(LEADER_2))
                .with_node(Node::new(LEADER_5).with_trusted_peer(LEADER_2))
                .with_node(Node::new(LEADER_6).with_trusted_peer(LEADER_3))
                .with_node(Node::new(LEADER_7).with_trusted_peer(LEADER_3)),
        )
        .blockchain_config(Blockchain::default().with_leaders(vec![
            LEADER_1, LEADER_2, LEADER_3, LEADER_4, LEADER_5, LEADER_6, LEADER_7,
        ]))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_000_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_7)
                .build(),
        )
        .build()
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

    FragmentSender::from(&controller.settings().block0)
        .send_transactions_round_trip(40, &mut wallet1, &mut wallet2, &leader1, 1_000.into())
        .unwrap();

    let leaders = [
        &leader1, &leader2, &leader3, &leader4, &leader5, &leader6, &leader7,
    ];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(7, 5).into(),
        "tree_sync",
        MeasurementReportInterval::Standard,
    )
    .unwrap();

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(7, 5).into(),
        "tree_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    );
}

#[test]
pub fn relay() {
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
        .blockchain_config(Blockchain::default().with_leaders(vec![
            CORE_NODE,
            RELAY_NODE_1,
            RELAY_NODE_2,
            LEADER_1,
            LEADER_2,
            LEADER_3,
            LEADER_4,
            LEADER_5,
            LEADER_6,
            LEADER_7,
        ]))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_000_000_000)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER_7)
                .build(),
        )
        .build()
        .unwrap();

    let core = controller
        .spawn(SpawnParams::new(CORE_NODE).in_memory())
        .unwrap();

    let relay1 = controller
        .spawn(SpawnParams::new(RELAY_NODE_1).in_memory())
        .unwrap();
    let relay2 = controller
        .spawn(SpawnParams::new(RELAY_NODE_2).in_memory())
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

    let setup = FragmentSenderSetup::resend_3_times_and_sync_with(vec![&core, &relay1, &relay2]);

    FragmentSender::from(&controller.settings().block0)
        .clone_with_setup(setup)
        .send_transactions_round_trip(5, &mut wallet1, &mut wallet2, &leader1, 1_000.into())
        .unwrap();

    let leaders = [
        &leader1, &leader2, &leader3, &leader4, &leader5, &leader6, &leader7, &relay1, &relay2,
        &core,
    ];

    utils::measure_and_log_sync_time(
        &leaders,
        SyncWaitParams::network_size(10, 3).into(),
        "relay_sync",
        MeasurementReportInterval::Standard,
    )
    .unwrap();

    utils::measure_single_transaction_propagation_speed(
        &mut controller,
        &mut wallet1,
        &wallet2,
        &leaders,
        SyncWaitParams::network_size(10, 3).into(),
        "relay_single_transaction_propagation",
        MeasurementReportInterval::Standard,
    );
}
