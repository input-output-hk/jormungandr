use crate::common::file_utils;
use crate::common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use jormungandr_lib::crypto::hash::Hash;

lazy_static! {
    static ref FAKE_INPUT_TRANSACTION_ID: Hash = {
        "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193"
            .parse()
            .unwrap()
    };
}
const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";

#[test]
pub fn test_cannot_create_input_with_negative_amount() {
    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH).assert_add_input_fail(
        &FAKE_INPUT_TRANSACTION_ID,
        0,
        "-100",
        "Found argument '-1' which wasn't expected",
    );
}

#[test]
pub fn test_cannot_create_input_with_too_big_utxo_amount() {
    JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH).assert_add_input_fail(
        &FAKE_INPUT_TRANSACTION_ID,
        0,
        "100000000000000000000",
        "error: Invalid value for '<value>': number too large to fit in target type",
    );
}

#[test]
#[cfg(not(target_os = "linux"))]
pub fn test_cannot_create_input_when_staging_file_is_readonly() {
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    file_utils::make_readonly(&transaction_wrapper.staging_file_path);
    transaction_wrapper.assert_add_input_fail(&FAKE_INPUT_TRANSACTION_ID, 0, "100", "denied");
}
