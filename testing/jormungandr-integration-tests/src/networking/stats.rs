use crate::common::{
    jcli_wrapper,
    network::{self, wallet},
    transaction_utils::TransactionHash,
};
const PASSIVE: &str = "PASSIVE";
const LEADER: &str = "LEADER";

#[test]
pub fn passive_node_last_block_info() {
    let mut network_controller = network::builder()
        .single_trust_direction(PASSIVE, LEADER)
        .initials(vec![
            wallet("alice").with(1_000_000).delegated_to(LEADER),
            wallet("bob").with(1_000_000),
        ])
        .build()
        .unwrap();

    let leader = network_controller.spawn_and_wait(LEADER);
    let passive = network_controller.spawn_as_passive_and_wait(PASSIVE);

    let mut alice = network_controller.wallet("alice").unwrap();
    let bob = network_controller.wallet("bob").unwrap();

    let stats_before = passive
        .rest()
        .stats()
        .expect("cannot get stats at beginning")
        .stats
        .expect("empty stats");
    for _ in 0..10 {
        let fragment = alice
            .transaction_to(
                &leader.genesis_block_hash(),
                &leader.fees(),
                bob.address(),
                10.into(),
            )
            .unwrap()
            .encode();
        jcli_wrapper::assert_transaction_in_block_with_wait(
            &fragment,
            &leader,
            &Default::default(),
        );

        alice.confirm_transaction();
    }

    let stats_after = passive
        .rest()
        .stats()
        .expect("cannot get stats at end")
        .stats
        .expect("empty stats");

    assert!(
        stats_before.last_block_content_size == stats_after.last_block_content_size,
        "last block content size should to be updated"
    );
    assert!(
        stats_before.last_block_date == stats_after.last_block_date,
        "last block date should to be updated"
    );
    assert!(
        stats_before.last_block_fees == stats_after.last_block_fees,
        "last block fees size should to be updated"
    );
    assert!(
        stats_before.last_block_hash == stats_after.last_block_hash,
        "last block hash should to be updated"
    );
    assert!(
        stats_before.last_block_sum == stats_after.last_block_sum,
        "last block sum should to be updated"
    );
    assert!(
        stats_before.last_block_time == stats_after.last_block_time,
        "last block time should to be updated"
    );
    assert!(
        stats_before.last_block_tx == stats_after.last_block_tx,
        "last block tx should to be updated"
    );
}
