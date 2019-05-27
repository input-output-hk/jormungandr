#![cfg(feature = "integration-test")]]

#[test]
#[cfg(not(target_os = "linux"))]
pub fn test_cannot_create_input_when_staging_file_is_readonly() {
    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(FAKE_GENESIS_HASH);
    file_utils::make_readonly(&transaction_wrapper.staging_file_path);
    transaction_wrapper.assert_add_input_fail(&FAKE_INPUT_TRANSACTION_ID, &0, "100", "denied");
}
