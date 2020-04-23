use crate::common::{
    jcli_wrapper,
    network::{builder, wallet},
    process_utils,
    transaction_utils::TransactionHash,
};
const PASSIVE: &str = "PASSIVE";
const LEADER: &str = "LEADER";

#[test]
pub fn passive_node_last_block_info() {
    let mut network_controller = builder("node_preffered_list_itself")
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

    println!("{:?}", passive.rest().stats());
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
            leader.process(),
            &Default::default(),
        );

        alice.confirm_transaction();

        process_utils::sleep(30);

        println!("{:?}", leader.rest().stats());
        println!("{:?}", passive.rest().stats());
    }
}
