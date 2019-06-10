#![cfg(feature = "integration-test")]

use common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;

const FAKE_INPUT_TRANSACTION_ID: &str =
    "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";

#[test]
pub fn test_cannot_add_utxo_id_as_output() {
    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH).assert_add_output_fail(
        &FAKE_INPUT_TRANSACTION_ID,
        &100,
        "invalid internal encoding",
    );
}
