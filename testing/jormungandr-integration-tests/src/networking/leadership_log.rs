use chain_impl_mockchain::chaintypes::ConsensusVersion;
use jormungandr_lib::interfaces::BlockDate;
use jormungandr_testing_utils::testing::network::blockchain::BlockchainBuilder;
use jormungandr_testing_utils::testing::network::builder::NetworkBuilder;
use jormungandr_testing_utils::testing::network::Node;
use jormungandr_testing_utils::testing::network::SpawnParams;
use jormungandr_testing_utils::testing::network::Topology;
use jormungandr_testing_utils::testing::node::time;
const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";

#[test]
pub fn leader_restart_preserves_leadership_log() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_1))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1)),
        )
        .blockchain_config(
            BlockchainBuilder::default()
                .consensus(ConsensusVersion::Bft)
                .slots_per_epoch(120)
                .slot_duration(2)
                .leader(LEADER_1)
                .leader(LEADER_2)
                .build(),
        )
        .build()
        .unwrap();

    let leader_1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();

    //wait more than half an epoch
    time::wait_for_date(BlockDate::new(0, 60), leader_1.rest());

    //start bft node 2
    let leader_2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();
    // logs during epoch 0 should be empty
    assert!(
        !leader_2.rest().leaders_log().unwrap().is_empty(),
        "leadership log should NOT be empty in current epoch",
    );

    time::wait_for_date(BlockDate::new(1, 0), leader_1.rest());

    // logs during epoch 0 should be empty
    assert!(
        !leader_2.rest().leaders_log().unwrap().is_empty(),
        "leadership log should NOT be empty in new epoch",
    );
}
