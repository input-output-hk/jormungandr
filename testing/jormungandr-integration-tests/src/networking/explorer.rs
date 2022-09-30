use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{BlockchainBuilder, SpawnParams, WalletTemplateBuilder},
};
use jormungandr_automation::{jormungandr::explorer::configuration::ExplorerParams, testing::time};
use jormungandr_lib::interfaces::BlockDate;
use thor::FragmentSender;
const LEADER_1: &str = "Leader_1";
const LEADER_2: &str = "Leader_2";
const LEADER_3: &str = "Leader_3";
const PASSIVE: &str = "Passive";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";
const CLARICE: &str = "CLARICE";

#[test]
pub fn passive_node_explorer() {
    let wait_epoch = 0;
    let wait_slot_id = 30;
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_1))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1))
                .with_node(Node::new(LEADER_3).with_trusted_peer(LEADER_1))
                .with_node(
                    Node::new(PASSIVE)
                        .with_trusted_peer(LEADER_1)
                        .with_trusted_peer(LEADER_2)
                        .with_trusted_peer(LEADER_3),
                ),
        )
        .blockchain_config(
            BlockchainBuilder::default()
                .slots_per_epoch(60)
                .slot_duration(2)
                .leaders(vec![LEADER_1, LEADER_2, LEADER_3])
                .build(),
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
        .wallet_template(
            WalletTemplateBuilder::new(CLARICE)
                .with(2_000_000_000)
                .delegated_to(LEADER_2)
                .build(),
        )
        .build()
        .unwrap();

    let leader_1 = controller
        .spawn(SpawnParams::new(LEADER_1).in_memory())
        .unwrap();
    let _leader_2 = controller
        .spawn(SpawnParams::new(LEADER_2).in_memory())
        .unwrap();
    let _leader_3 = controller
        .spawn(SpawnParams::new(LEADER_3).in_memory())
        .unwrap();

    let passive = controller
        .spawn(SpawnParams::new(PASSIVE).passive().in_memory())
        .unwrap();
    let mut alice = controller.controlled_wallet(ALICE).unwrap();
    let bob = controller.controlled_wallet(BOB).unwrap();

    let mem_pool_check = FragmentSender::from(&controller.settings().block0)
        .send_transaction(&mut alice, &bob, &leader_1, 1_000.into())
        .unwrap();

    // give some time to update explorer
    time::wait_for_date(BlockDate::new(wait_epoch, wait_slot_id), leader_1.rest());

    let transaction_id = passive
        .explorer(ExplorerParams::default())
        .unwrap()
        .client()
        .transaction((*mem_pool_check.fragment_id()).into())
        .unwrap()
        .data
        .unwrap()
        .transaction
        .id;

    assert_eq!(
        &transaction_id,
        &mem_pool_check.fragment_id().to_string(),
        "Wrong transaction id in explorer",
    );
}
