use crate::test::non_functional::*;
use jormungandr_testing_utils::testing::network::blockchain::BlockchainBuilder;
use jormungandr_testing_utils::testing::network::builder::NetworkBuilder;
use jormungandr_testing_utils::testing::network::wallet::template::builder::WalletTemplateBuilder;
use jormungandr_testing_utils::testing::network::Node;
use jormungandr_testing_utils::testing::network::SpawnParams;
use jormungandr_testing_utils::testing::network::Topology;
use jormungandr_testing_utils::testing::FragmentSender;
use jormungandr_testing_utils::testing::SyncWaitParams;
use jormungandr_testing_utils::testing::{ensure_nodes_are_in_sync, FragmentVerifier};
use std::time::{Duration, SystemTime};

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

    let mut wallet1 = controller.wallet(ALICE).unwrap();
    let mut wallet2 = controller.wallet(BOB).unwrap();
    let mut wallet3 = controller.wallet(CLARICE).unwrap();
    let mut wallet4 = controller.wallet(DAVID).unwrap();
    let mut wallet5 = controller.wallet(EDGAR).unwrap();
    let mut wallet6 = controller.wallet(FILIP).unwrap();
    let mut wallet7 = controller.wallet(GRACE).unwrap();

    let now = SystemTime::now();

    let fragment_sender = FragmentSender::from(controller.settings());

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
