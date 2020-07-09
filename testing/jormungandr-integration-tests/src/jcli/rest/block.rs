use crate::common::{jcli_wrapper, jormungandr::starter::Starter};
use assert_cmd::assert::OutputAssertExt;
#[test]
pub fn test_non_empty_hash_is_returned_for_block0() {
    let jormungandr = Starter::new().start().unwrap();
    let rest_uri = jormungandr.rest_uri();
    let block_id = jcli_wrapper::assert_rest_get_block_tip(&rest_uri);
    let actual_hash = jcli_wrapper::assert_rest_get_block_by_id(&block_id, &rest_uri);

    assert_ne!(&actual_hash, "", "empty block hash");
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_block_id() {
    let incorrect_block_id = "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641aa";
    let jormungandr = Starter::new().start().unwrap();

    jcli_wrapper::jcli_commands::get_rest_get_block_command(
        &incorrect_block_id,
        &jormungandr.rest_uri(),
    )
    .assert()
    .failure()
    .stderr(predicates::str::contains(
        "node rejected request because of invalid parameters",
    ));
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_block_id_in_next_block_id_request() {
    let incorrect_block_id = "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641aa";

    let jormungandr = Starter::new().start().unwrap();

    jcli_wrapper::jcli_commands::get_rest_get_next_block_id_command(
        &incorrect_block_id,
        1,
        &jormungandr.rest_uri(),
    )
    .assert()
    .failure()
    .stderr(predicates::str::contains(
        "node rejected request because of invalid parameters",
    ));
}
