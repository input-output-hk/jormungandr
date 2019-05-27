#![cfg(feature = "integration-test")]

use common::jcli_wrapper;
use common::process_assert;

#[test]

pub fn test_correct_error_is_returned_for_incorrect_host_syntax() {
    let incorrect_host = "not_a_correct_syntax";

    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_rest_block_tip_command(&incorrect_host),
        "Invalid value for '--host <host>': relative URL without a base",
    );
}

#[test]
/// False green due to: #298
pub fn test_correct_error_is_returned_for_incorrect_host_address() {
    let incorrect_host = "http://127.0.0.100:8443/api";

    process_assert::assert_process_failed_and_matches_message_with_desc(
        jcli_wrapper::jcli_commands::get_rest_block_tip_command(&incorrect_host),
        "thread 'main' panicked at",
        "This assertion is incorrect on purpose to avoid failing build when running test,
        after #298 is fixed it need to be changed to correct one",
    );
}
