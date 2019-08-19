use crate::common::jcli_wrapper;
use crate::common::process_assert;

#[test]

pub fn test_correct_error_is_returned_for_incorrect_host_syntax() {
    let incorrect_host = "not_a_correct_syntax";

    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_rest_block_tip_command(&incorrect_host),
        "Invalid value for '--host <host>': relative URL without a base",
    );
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_host_address() {
    // Port 9 is standard port discarding all requests
    let incorrect_host = "http://127.0.0.1:9/api";

    process_assert::assert_process_failed_and_matches_message(
        jcli_wrapper::jcli_commands::get_rest_block_tip_command(&incorrect_host),
        "could not connect with node",
    );
}
