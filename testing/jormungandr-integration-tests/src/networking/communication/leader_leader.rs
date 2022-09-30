use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{Blockchain, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::testing::{
    benchmark::sync::{measure_and_log_sync_time, MeasurementReportInterval},
    SyncWaitParams,
};
use thor::FragmentSender;

const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";

#[test]
pub fn two_transaction_to_two_leaders() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_2))
                .with_node(Node::new(LEADER_1).with_trusted_peer(LEADER_2)),
        )
        .blockchain_config(Blockchain::default().with_leaders(vec![LEADER_1, LEADER_2]))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .delegated_to(LEADER_2)
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

    let leader_2 = controller
        .spawn(SpawnParams::new(LEADER_2).in_memory())
        .unwrap();

    let leader_1 = controller
        .spawn(SpawnParams::new(LEADER_1).in_memory())
        .unwrap();

    let mut alice = controller.controlled_wallet(ALICE).unwrap();
    let mut bob = controller.controlled_wallet(BOB).unwrap();

    let fragment_sender = FragmentSender::from(&controller.settings().block0);

    for _ in 0..10 {
        fragment_sender
            .send_transaction(&mut alice, &bob, &leader_1, 1_000.into())
            .unwrap();
        fragment_sender
            .send_transaction(&mut bob, &alice, &leader_2, 1_000.into())
            .unwrap();
    }

    measure_and_log_sync_time(
        &[&leader_1, &leader_2],
        SyncWaitParams::two_nodes().into(),
        "two_transaction_to_two_leaders_sync",
        MeasurementReportInterval::Standard,
    )
    .unwrap();
}
