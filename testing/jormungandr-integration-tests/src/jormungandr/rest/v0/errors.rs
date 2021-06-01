use crate::common::jormungandr::JormungandrProcess;
use crate::common::{jormungandr::ConfigurationBuilder, startup};
use jormungandr_lib::interfaces::FragmentStatus;
use jormungandr_testing_utils::testing::FragmentSenderSetup;
use jormungandr_testing_utils::testing::FragmentVerifier;
use jormungandr_testing_utils::testing::MemPoolCheck;
use jormungandr_testing_utils::wallet::Wallet;
use rstest::*;
use std::time::Duration;

#[fixture]
fn world() -> (JormungandrProcess, Wallet, Wallet, Wallet) {
    let alice = startup::create_new_account_address();
    let bob = startup::create_new_account_address();
    let clarice = startup::create_new_account_address();

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[bob.clone()],
        &mut ConfigurationBuilder::new(),
    )
    .unwrap();

    (jormungandr, alice, bob, clarice)
}

#[rstest]
pub fn fragment_already_in_log(world: (JormungandrProcess, Wallet, Wallet, Wallet)) {
    let (jormungandr, mut alice, bob, _) = world;

    let transaction_sender = jormungandr.fragment_sender(FragmentSenderSetup::resend_3_times());
    let invalid_transaction_sender = jormungandr.fragment_sender(FragmentSenderSetup::no_verify());

    let alice_fragment = alice
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            bob.address(),
            100.into(),
        )
        .unwrap();

    let tx_id = transaction_sender
        .send_fragment(&mut alice, alice_fragment.clone(), &jormungandr)
        .unwrap();

    let verifier = FragmentVerifier;
    verifier
        .wait_and_verify_is_in_block(Duration::from_secs(5), tx_id, &jormungandr)
        .unwrap();

    assert!(jormungandr.rest().send_fragment(alice_fragment.clone()))
        .err()
        .to_string()
        .contains("already in the log");
}

/*
#[rstest]
pub fn pool_overflow(world: (JormungandrProcess, FragmentId, FragmentId, FragmentId)) {
    let (jormungandr, alice_tx_id, bob_tx_id, _) = world;

    assert_multiple_ids(
        vec![alice_tx_id.to_string(), bob_tx_id.to_string()],
        "alice or bob tx",
        &jormungandr,
    );
}
*/
