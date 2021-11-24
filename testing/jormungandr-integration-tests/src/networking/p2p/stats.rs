use crate::networking::utils;
use jormungandr_lib::interfaces::Policy;
use jormungandr_testing_utils::testing::network::builder::NetworkBuilder;
use jormungandr_testing_utils::testing::network::wallet::template::builder::WalletTemplateBuilder;
use jormungandr_testing_utils::testing::network::Node;
use jormungandr_testing_utils::testing::network::SpawnParams;
use jormungandr_testing_utils::testing::network::Topology;
use std::time::Duration;

const LEADER1: &str = "LEADER1";
const LEADER2: &str = "LEADER2";
const LEADER3: &str = "LEADER3";
const LEADER4: &str = "LEADER4";

const ALICE: &str = "ALICE";
const BOB: &str = "BOB";
const CLARICE: &str = "CLARICE";

#[test]
pub fn p2p_stats_test() {
    let mut controller = NetworkBuilder::default()
        .topology(
            Topology::default()
                .with_node(Node::new(LEADER1))
                .with_node(Node::new(LEADER2).with_trusted_peer(LEADER1))
                .with_node(Node::new(LEADER3).with_trusted_peer(LEADER1))
                .with_node(
                    Node::new(LEADER4)
                        .with_trusted_peer(LEADER2)
                        .with_trusted_peer(LEADER3),
                ),
        )
        .wallet_template(
            WalletTemplateBuilder::new(ALICE)
                .with(2_000_000_000)
                .delegated_to(LEADER1)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(BOB)
                .with(2_000_000_000)
                .delegated_to(LEADER2)
                .build(),
        )
        .wallet_template(
            WalletTemplateBuilder::new(CLARICE)
                .with(2_000_000_000)
                .delegated_to(LEADER3)
                .build(),
        )
        .build()
        .unwrap();

    let policy = Policy {
        quarantine_duration: Some(Duration::new(120, 0).into()),
        quarantine_whitelist: None,
    };

    let leader1 = controller
        .spawn(SpawnParams::new(LEADER1).in_memory().policy(policy.clone()))
        .unwrap();

    super::assert_node_stats(&leader1, 0, 0, 0, "no peers for leader1");
    let info_before = "no peers for leader 1";
    assert!(
        leader1.rest().network_stats().unwrap().is_empty(),
        "{} network_stats",
        info_before,
    );
    assert!(
        leader1.rest().p2p_quarantined().unwrap().is_empty(),
        "{} p2p_quarantined",
        info_before,
    );
    assert!(
        leader1.rest().p2p_non_public().unwrap().is_empty(),
        "{} p2p_non_public",
        info_before,
    );
    assert!(
        leader1.rest().p2p_available().unwrap().is_empty(),
        "{} p2p_available",
        info_before,
    );
    assert!(
        leader1.rest().p2p_view().unwrap().is_empty(),
        "{} p2p_view",
        info_before,
    );

    let leader2 = controller
        .spawn(
            SpawnParams::new(LEADER2)
                .in_memory()
                .no_listen_address()
                .policy(policy.clone()),
        )
        .unwrap();

    utils::wait(20);
    super::assert_node_stats(&leader1, 1, 0, 1, "bootstrapped leader1");
    super::assert_node_stats(&leader2, 1, 0, 1, "bootstrapped leader2");

    let leader3 = controller
        .spawn(
            SpawnParams::new(LEADER3)
                .in_memory()
                .no_listen_address()
                .policy(policy.clone()),
        )
        .unwrap();

    utils::wait(20);
    super::assert_node_stats(&leader1, 2, 0, 2, "leader1: leader3 node is up");
    super::assert_node_stats(&leader2, 2, 0, 2, "leader2: leader3 node is up");
    super::assert_node_stats(&leader3, 2, 0, 2, "leader3: leader3 node is up");

    let leader4 = controller
        .spawn(
            SpawnParams::new(LEADER4)
                .in_memory()
                .no_listen_address()
                .policy(policy),
        )
        .unwrap();

    utils::wait(20);
    super::assert_node_stats(&leader1, 3, 0, 3, "leader1: leader4 node is up");
    super::assert_node_stats(&leader2, 3, 0, 3, "leader2: leader4 node is up");
    super::assert_node_stats(&leader3, 3, 0, 3, "leader3: leader4 node is up");
    super::assert_node_stats(&leader3, 3, 0, 3, "leader4: leader4 node is up");

    leader2.shutdown();
    utils::wait(20);
    //TODO try to determine why quarantine counter id not bumped up
    super::assert_node_stats(&leader1, 3, 0, 3, "leader1: leader 2 is down");
    super::assert_node_stats(&leader3, 3, 0, 3, "leader3: leader 2 is down");
    super::assert_node_stats(&leader4, 3, 0, 3, "leader4: leader 2 is down")
}
