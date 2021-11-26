use jormungandr_testing_utils::testing::network::Blockchain;
use jormungandr_testing_utils::testing::network::{
    builder::NetworkBuilder, wallet::template::builder::WalletTemplateBuilder,
};
use jormungandr_testing_utils::testing::network::{Node, SpawnParams, Topology};
use jormungandr_testing_utils::testing::MemPoolCheck;

const PASSIVE: &str = "PASSIVE";
const LEADER: &str = "LEADER";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";

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

    let mut alice = network_controller.wallet(ALICE).unwrap();
    let mut bob = network_controller.wallet(BOB).unwrap();

    passive
        .fragment_sender(Default::default())
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
