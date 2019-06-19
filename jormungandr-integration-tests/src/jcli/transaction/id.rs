#![cfg(feature = "integration-test")]

use common::file_utils;
use common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;

const FAKE_INPUT_TRANSACTION_ID: &str =
    "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_ACCOUNT_ADDRESS: &str = "ta1s5fzaewqt7yq89vma8atryks9vyvfacr5hq8wm64vhn36f62lyvrzuxm7dm";

#[test]
pub fn test_get_id_for_new_transaction() {
    let transaction_id =
        JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH).get_transaction_id();
    assert_ne!(&transaction_id, "", "transaction id is empty");
}

#[test]
pub fn test_get_id_for_transaction_with_single_input() {
    let transaction_id = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH)
        .add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .get_transaction_id();
    assert_ne!(&transaction_id, "", "transaction id is empty");
}

#[test]
pub fn test_get_id_for_finalized_transaction() {
    let transaction_id = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH)
        .add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .add_output(&FAKE_ACCOUNT_ADDRESS, &100)
        .assert_finalize()
        .get_transaction_id();
    assert_ne!(&transaction_id, "", "transaction id is empty");
}

#[test]
pub fn test_get_id_for_sealed_transaction() {
    let sender = startup::create_new_utxo_address();
    let transaction_id = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&reciever.address, &transfer_amount)
        .assert_finalize()
        .seal_with_witness_deafult(&sender.private_key, "utxo")
        .get_transaction_id();
    assert_ne!(&transaction_id, "", "transaction id is empty");
}
