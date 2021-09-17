use jormungandr_testing_utils::testing::common::jcli::JCli;

#[test]

pub fn test_correct_error_is_returned_for_incorrect_host_syntax() {
    let jcli: JCli = Default::default();
    let incorrect_host = "not_a_correct_syntax";

    jcli.rest().v0().tip_expect_fail(
        incorrect_host,
        "Invalid value for '--host <host>': relative URL without a base",
    );
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_host_address() {
    let jcli: JCli = Default::default();
    // Port 9 is standard port discarding all requests
    let incorrect_host = "http://127.0.0.1:9/api";
    jcli.rest()
        .v0()
        .tip_expect_fail(incorrect_host, "tcp connect error");
}
