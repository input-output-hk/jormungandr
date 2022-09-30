use chain_impl_mockchain::chaintypes::ConsensusVersion;
use hersir::{
    builder::{NetworkBuilder, Node, Topology},
    config::{BlockchainBuilder, SpawnParams},
};
use jormungandr_automation::testing::time;
use jormungandr_lib::interfaces::BlockDate;
const LEADER_1: &str = "Leader1";
const LEADER_2: &str = "Leader2";

#[test]
pub fn leader_late_start_preserves_leadership_log() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER_1))
                .with_node(Node::new(LEADER_2).with_trusted_peer(LEADER_1)),
        )
        .blockchain_config(
            BlockchainBuilder::default()
                .consensus(ConsensusVersion::Bft)
                .slots_per_epoch(60)
                .slot_duration(2)
                .leader(LEADER_1)
                .leader(LEADER_2)
                .build(),
        )
        .build()
        .unwrap();

    let leader_1 = controller.spawn(SpawnParams::new(LEADER_1)).unwrap();

    //wait more than half an epoch
    time::wait_for_date(BlockDate::new(0, 40), leader_1.rest());

    //start bft node 2
    let leader_2 = controller.spawn(SpawnParams::new(LEADER_2)).unwrap();

    // wait a bit so leader 2 has time to produce blocks
    time::wait_for_date(BlockDate::new(0, 90), leader_1.rest());

    // logs during epoch 0 should not be empty
    assert!(
        !leader_2.rest().leaders_log().unwrap().is_empty(),
        "leadership log should NOT be empty in current epoch",
    );

    time::wait_for_date(BlockDate::new(1, 0), leader_1.rest());

    // logs during epoch 1 should not be empty
    assert!(
        !leader_2.rest().leaders_log().unwrap().is_empty(),
        "leadership log should NOT be empty in new epoch",
    );
}
