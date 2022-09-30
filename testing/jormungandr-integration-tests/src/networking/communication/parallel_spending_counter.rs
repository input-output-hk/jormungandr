use chain_impl_mockchain::testing::WitnessMode;
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{Blockchain, SpawnParams, WalletTemplateBuilder},
};
use thor::{FragmentSender, FragmentSenderSetup, FragmentVerifier};

const LEADER: &str = "Leader";
const PASSIVE_1: &str = "Passive1";
const PASSIVE_2: &str = "Passive2";
const PASSIVE_3: &str = "Passive3";
const PASSIVE_4: &str = "Passive4";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";

use rand::{seq::SliceRandom, thread_rng};

#[test]
pub fn account_send_4_parallel_transaction_through_4_proxies() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER))
                .with_node(Node::new(PASSIVE_1).with_trusted_peer(LEADER))
                .with_node(Node::new(PASSIVE_2).with_trusted_peer(LEADER))
                .with_node(Node::new(PASSIVE_3).with_trusted_peer(LEADER))
                .with_node(Node::new(PASSIVE_4).with_trusted_peer(LEADER)),
        )
        .blockchain_config(Blockchain::default().with_leader(LEADER))
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_500_000_000)
                .delegated_to(LEADER)
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

    let leader = controller
        .spawn(SpawnParams::new(LEADER).in_memory())
        .unwrap();

    let passive_1 = controller
        .spawn(SpawnParams::new(PASSIVE_1).in_memory())
        .unwrap();

    let passive_2 = controller
        .spawn(SpawnParams::new(PASSIVE_2).in_memory())
        .unwrap();

    let passive_3 = controller
        .spawn(SpawnParams::new(PASSIVE_3).in_memory())
        .unwrap();

    let passive_4 = controller
        .spawn(SpawnParams::new(PASSIVE_4).in_memory())
        .unwrap();

    let mut alice = controller.controlled_wallet(ALICE).unwrap();
    let bob = controller.controlled_wallet(BOB).unwrap();

    let mut fragment_sender = FragmentSender::from(&controller.settings().block0)
        .clone_with_setup(FragmentSenderSetup::no_verify());
    let mut checks = vec![];

    let mut lanes: Vec<usize> = (1..=4).collect();
    lanes.shuffle(&mut thread_rng());

    fragment_sender = fragment_sender.witness_mode(WitnessMode::Account { lane: lanes[0] });
    checks.push(
        fragment_sender
            .send_transaction(&mut alice, &bob, &passive_1, 1.into())
            .unwrap(),
    );

    fragment_sender = fragment_sender.witness_mode(WitnessMode::Account { lane: lanes[1] });
    checks.push(
        fragment_sender
            .send_transaction(&mut alice, &bob, &passive_2, 1.into())
            .unwrap(),
    );

    fragment_sender = fragment_sender.witness_mode(WitnessMode::Account { lane: lanes[2] });
    checks.push(
        fragment_sender
            .send_transaction(&mut alice, &bob, &passive_3, 1.into())
            .unwrap(),
    );

    fragment_sender = fragment_sender.witness_mode(WitnessMode::Account { lane: lanes[3] });
    checks.push(
        fragment_sender
            .send_transaction(&mut alice, &bob, &passive_4, 1.into())
            .unwrap(),
    );

    FragmentVerifier::wait_and_verify_all_are_in_block(
        std::time::Duration::from_secs(10),
        checks,
        &leader,
    )
    .unwrap();
}
