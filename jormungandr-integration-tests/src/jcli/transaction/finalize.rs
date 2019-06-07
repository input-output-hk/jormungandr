use crate::common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use crate::common::startup;

const FAKE_INPUT_TRANSACTION_ID: &str =
    "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_ACCOUNT_ADDRESS: &str = "ta1s5fzaewqt7yq89vma8atryks9vyvfacr5hq8wm64vhn36f62lyvrzuxm7dm";

#[test]
pub fn test_unbalanced_output_utxo_transation_is_not_finalized() {
    let reciever = startup::create_new_utxo_address();

    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH)
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_add_output(&reciever.address, &150)
        .assert_finalize_fail("not enough input for making transaction");
}

#[test]
pub fn test_cannot_finalize_empty_transaction() {
    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH)
        .assert_finalize_fail("transaction has no inputs");
}

#[test]
pub fn test_cannot_finalize_transaction_with_single_input() {
    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH)
        .assert_add_input(&FAKE_INPUT_TRANSACTION_ID, &0, &100)
        .assert_finalize_fail("transaction has no outputs");
}

#[test]
pub fn test_cannot_finalize_transaction_with_single_account() {
    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH)
        .assert_add_account(&FAKE_ACCOUNT_ADDRESS, &100)
        .assert_finalize_fail("transaction has no outputs");
}

#[test]
pub fn test_cannot_finalize_transaction_with_single_output() {
    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH)
        .assert_add_output(&FAKE_ACCOUNT_ADDRESS, &100)
        .assert_finalize_fail("transaction has no inputs");
}
