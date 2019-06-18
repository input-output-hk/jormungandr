#![cfg(feature = "integration-test")]

use common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use common::startup;

const FAKE_INPUT_TRANSACTION_ID: &str =
    "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_ACCOUNT_ADDRESS: &str = "ta1s5fzaewqt7yq89vma8atryks9vyvfacr5hq8wm64vhn36f62lyvrzuxm7dm";

#[test]
pub fn test_cannot_seal_transaction_with_no_witness() {
    let reciever = startup::create_new_utxo_address();
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);

    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_seal_fail("cannot seal, not enough witnesses");
}

#[test]
pub fn test_cannot_seal_transaction_with_too_few_witnesses() {
    let reciever = startup::create_new_utxo_address();

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    let witness = transaction_wrapper.create_witness_default("utxo");

    transaction_wrapper
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &100)
        .assert_finalize()
        .assert_make_witness(&witness)
        .assert_add_witness(&witness)
        .assert_seal_fail("cannot seal, not enough witnesses");
}
