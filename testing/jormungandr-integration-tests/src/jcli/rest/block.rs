use jormungandr_testing_utils::testing::common::{jcli::JCli, jormungandr::starter::Starter};

#[test]
pub fn test_non_empty_hash_is_returned_for_block0() {
    let jcli: JCli = Default::default();
    let jormungandr = Starter::new().start().unwrap();
    let rest_uri = jormungandr.rest_uri();
    let block_id = jcli.rest().v0().tip(&rest_uri);
    jcli.rest().v0().block().get(block_id, rest_uri);
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_block_id() {
    let jcli: JCli = Default::default();
    let incorrect_block_id = "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641aa";
    let jormungandr = Starter::new().start().unwrap();

    jcli.rest().v0().block().get_expect_fail(
        incorrect_block_id,
        jormungandr.rest_uri(),
        "node rejected request because of invalid parameters",
    );
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_block_id_in_next_block_id_request() {
    let jcli: JCli = Default::default();
    let incorrect_block_id = "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641aa";

    let jormungandr = Starter::new().start().unwrap();

    jcli.rest().v0().block().next_expect_fail(
        incorrect_block_id,
        1,
        jormungandr.rest_uri(),
        "node rejected request because of invalid parameters",
    );
}
