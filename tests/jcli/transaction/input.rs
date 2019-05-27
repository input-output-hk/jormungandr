#![cfg(feature = "integration-test")]

use common::file_utils;
use common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;

const FAKE_INPUT_TRANSACTION_ID: &str =
    "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";

#[test]
#[cfg(feature = "integration-test")]
pub fn test_cannot_create_input_with_negative_amount() {
    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH).assert_add_input_fail(
        &FAKE_INPUT_TRANSACTION_ID,
        &0,
        "-100",
        "Found argument '-1' which wasn't expected",
    );
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_cannot_create_input_with_too_big_utxo_amount() {
    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH).assert_add_input_fail(
        &FAKE_INPUT_TRANSACTION_ID,
        &0,
        "100000000000000000000",
        "error: Invalid value for '<value>': Invalid value",
    );
}

#[test]
#[cfg(not(target_os = "linux"))]
pub fn test_cannot_create_input_when_staging_file_is_readonly() {
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    file_utils::make_readonly(&transaction_wrapper.staging_file_path);
    transaction_wrapper.assert_add_input_fail(&FAKE_INPUT_TRANSACTION_ID, &0, "100", "denied");
}
