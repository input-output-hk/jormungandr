use crate::test::utils;
use jormungandr_testing_utils::testing::network::builder::NetworkBuilder;
use jormungandr_testing_utils::testing::network::Node;
use jormungandr_testing_utils::testing::network::SpawnParams;
use jormungandr_testing_utils::testing::network::Topology;
const LEADER1: &str = "LEADER1";
const LEADER2: &str = "LEADER2";
const LEADER3: &str = "LEADER3";
const LEADER4: &str = "LEADER4";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";
const CLARICE: &str = "CLARICE";

#[test]
pub fn max_connections() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER1))
                .with_node(Node::new(LEADER2).with_trusted_peer(LEADER1))
                .with_node(Node::new(LEADER3).with_trusted_peer(LEADER1))
                .with_node(Node::new(LEADER4).with_trusted_peer(LEADER1)),
        )
        .build()
        .unwrap();

    let leader1 = controller
        .spawn(
            SpawnParams::new(LEADER1)
                .in_memory()
                .max_inbound_connections(2),
        )
        .unwrap();

    let _leader2 = controller
        .spawn(SpawnParams::new(LEADER2).in_memory())
        .unwrap();

    let _leader3 = controller
        .spawn(SpawnParams::new(LEADER3).in_memory())
        .unwrap();

    let _leader4 = controller
        .spawn(SpawnParams::new(LEADER4).in_memory())
        .unwrap();

    utils::wait(30);
    super::assert_connected_cnt(&leader1, 2, "leader1 should have only 2 nodes connected");
}
