use chain_impl_mockchain::{block::BlockDate, fragment::FragmentId};
use jormungandr_testing_utils::testing::jormungandr::JormungandrProcess;
use jormungandr_testing_utils::testing::{jormungandr::ConfigurationBuilder, startup};
use jormungandr_testing_utils::testing::{FragmentSenderSetup, MemPoolCheck};
use rstest::*;

#[fixture]
fn world() -> (JormungandrProcess, FragmentId, FragmentId, FragmentId) {
    let mut alice = startup::create_new_account_address();
    let mut bob = startup::create_new_account_address();
    let mut clarice = startup::create_new_account_address();

    let (jormungandr, _stake_pools) = startup::start_stake_pool(
        &[alice.clone()],
        &[bob.clone()],
        &mut ConfigurationBuilder::new(),
    )
    .unwrap();

    let transaction_sender = jormungandr.fragment_sender(FragmentSenderSetup::resend_3_times());

    let alice_fragment = alice
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            bob.address(),
            100.into(),
        )
        .unwrap();

    let bob_fragment = bob
        .transaction_to(
            &jormungandr.genesis_block_hash(),
            &jormungandr.fees(),
            BlockDate::first().next_epoch(),
            alice.address(),
            100.into(),
        )
        .unwrap();

    let clarice_tx = transaction_sender
        .clone_with_setup(FragmentSenderSetup::no_verify())
        .send_transaction(&mut clarice, &bob, &jormungandr, 100.into())
        .unwrap();

    let summary = transaction_sender
        .send_batch_fragments(vec![alice_fragment, bob_fragment], false, &jormungandr)
        .unwrap();

    let tx_ids: Vec<MemPoolCheck> = summary
        .fragment_ids()
        .into_iter()
        .map(MemPoolCheck::from)
        .collect();

    tx_ids
        .iter()
        .for_each(|x| transaction_sender.verify(x, &jormungandr).unwrap());

    let alice_tx_id = tx_ids[0].fragment_id();
    let bob_tx_id = tx_ids[1].fragment_id();
    let clarice_tx_id = clarice_tx.fragment_id();

    (jormungandr, *alice_tx_id, *bob_tx_id, *clarice_tx_id)
}

#[rstest]
pub fn test_single_id(world: (JormungandrProcess, FragmentId, FragmentId, FragmentId)) {
    let (jormungandr, alice_tx_id, _, _) = world;
    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_single_id(alice_tx_id.to_string(), "alice tx");
}

#[rstest]
pub fn test_multiple_ids(world: (JormungandrProcess, FragmentId, FragmentId, FragmentId)) {
    let (jormungandr, alice_tx_id, bob_tx_id, _) = world;

    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_multiple_ids(
            vec![alice_tx_id.to_string(), bob_tx_id.to_string()],
            "alice or bob tx",
        );
}

#[rstest]
pub fn test_empty_ids(world: (JormungandrProcess, FragmentId, FragmentId, FragmentId)) {
    let (jormungandr, _, _, _) = world;
    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_empty_ids(vec![], "no tx");
}

#[rstest]
pub fn test_invalid_id(world: (JormungandrProcess, FragmentId, FragmentId, FragmentId)) {
    let (jormungandr, _, _, clarice_tx_id) = world;
    jormungandr
        .correct_state_verifier()
        .fragment_logs()
        .assert_invalid_id(clarice_tx_id.to_string(), "invalid clarice tx");
}
