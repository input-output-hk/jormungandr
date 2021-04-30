use crate::common::jormungandr::JormungandrProcess;
use crate::common::{jormungandr::ConfigurationBuilder, startup};
use assert_fs::prelude::*;
use assert_fs::TempDir;
use jormungandr_lib::interfaces::FragmentStatus;
use jormungandr_testing_utils::testing::{FragmentSender, FragmentSenderSetup};

#[test]
pub fn test_v1_endpoint() {
    let temp_dir = TempDir::new().unwrap();

    let mut alice = startup::create_new_account_address();
    let mut bob = startup::create_new_account_address();
    let mut clarice = startup::create_new_account_address();

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[bob.clone()],
        &mut ConfigurationBuilder::new().with_storage(&temp_dir.child("storage")),
    )
    .unwrap();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::resend_3_times(),
    );

    let alice_fragment = alice
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            bob.address(),
            100.into(),
        )
        .unwrap();

    let bob_fragment = bob
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            alice.address(),
            100.into(),
        )
        .unwrap();

    let tx_ids = transaction_sender
        .send_batch_fragments(vec![alice_fragment, bob_fragment], false, &jormungandr)
        .unwrap();

    tx_ids
        .iter()
        .for_each(|x| transaction_sender.verify(x, &jormungandr).unwrap());
    let alice_tx_id = tx_ids[0].fragment_id().to_string();
    let bob_tx_id = tx_ids[1].fragment_id().to_string();

    assert_single_id(alice_tx_id.clone(), "alice tx", &jormungandr);
    assert_multiple_ids(
        vec![alice_tx_id.clone(), bob_tx_id.clone()],
        "alice or bob tx",
        &jormungandr,
    );
    assert_empty_ids(vec![], "no tx", &jormungandr);

    // invalid tx
    let clarice_tx = transaction_sender
        .clone_with_setup(FragmentSenderSetup::no_verify())
        .send_transaction(&mut clarice, &bob, &jormungandr, 100.into())
        .unwrap();

    let clarice_tx_id = clarice_tx.fragment_id().to_string();
    assert_invalid_id(clarice_tx_id, "invalid clarice tx", &jormungandr);
}

fn assert_invalid_id(id: String, prefix: &str, jormungandr: &JormungandrProcess) {
    let statuses = jormungandr
        .rest()
        .fragments_statuses(vec![id.clone()])
        .unwrap();
    assert_eq!(1, statuses.len());

    let invalid_id = statuses.get(&id);

    match invalid_id {
        Some(status) => assert_not_in_block(status),
        None => panic!("Assert Error: {}", prefix),
    }
}

fn assert_single_id(id: String, prefix: &str, jormungandr: &JormungandrProcess) {
    let statuses = jormungandr
        .rest()
        .fragments_statuses(vec![id.clone()])
        .unwrap();

    assert_eq!(1, statuses.len());

    let alice_tx_status = statuses.get(&id);

    match alice_tx_status {
        Some(status) => assert_in_block(status),
        None => panic!("Assert Error: {}", prefix),
    }
}

fn assert_multiple_ids(ids: Vec<String>, prefix: &str, jormungandr: &JormungandrProcess) {
    let statuses = jormungandr.rest().fragments_statuses(ids.clone()).unwrap();

    assert_eq!(ids.len(), statuses.len());

    ids.iter().for_each(|id| match statuses.get(id) {
        Some(status) => assert_in_block(status),
        None => panic!("{}", prefix),
    })
}

fn assert_empty_ids(ids: Vec<String>, prefix: &str, jormungandr: &JormungandrProcess) {
    assert!(
        jormungandr.rest().fragments_statuses(ids).is_err(),
        "{} - expected failure",
        prefix
    );
}

fn assert_in_block(fragment_status: &FragmentStatus) {
    match fragment_status {
        FragmentStatus::InABlock { .. } => (),
        _ => panic!("should be in block '{:?}'", fragment_status),
    }
}

fn assert_not_in_block(fragment_status: &FragmentStatus) {
    match fragment_status {
        FragmentStatus::InABlock { .. } => panic!("should NOT be in block '{:?}'", fragment_status),
        _ => (),
    }
}
