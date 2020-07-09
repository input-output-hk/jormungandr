use crate::common::jcli_wrapper;
use assert_cmd::assert::OutputAssertExt;

#[test]

pub fn test_correct_error_is_returned_for_incorrect_host_syntax() {
    let incorrect_host = "not_a_correct_syntax";

    jcli_wrapper::jcli_commands::get_rest_block_tip_command(&incorrect_host)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "Invalid value for '--host <host>': relative URL without a base",
        ));
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_host_address() {
    // Port 9 is standard port discarding all requests
    let incorrect_host = "http://127.0.0.1:9/api";

    jcli_wrapper::jcli_commands::get_rest_block_tip_command(&incorrect_host)
        .assert()
        .failure()
        .stderr(predicates::str::contains("tcp connect error"));
}
