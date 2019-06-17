use crate::common::file_utils;
use crate::common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use crate::common::startup;

const FAKE_GENESIS_HASH: &str = "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";
const FAKE_INPUT_TRANSACTION_ID: &str =
    "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193";

#[test]
#[cfg(not(target_os = "linux"))]
pub fn test_cannot_create_input_when_staging_file_is_readonly() {
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    file_utils::make_readonly(&transaction_wrapper.staging_file_path);
    transaction_wrapper.assert_add_input_fail(&FAKE_INPUT_TRANSACTION_ID, &0, "100", "denied");
}
