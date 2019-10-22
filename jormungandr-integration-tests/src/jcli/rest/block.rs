use crate::common::{jcli_wrapper, jormungandr::starter::Starter, process_assert};

#[test]
pub fn test_non_empty_hash_is_returned_for_block0() {
    let jormungandr = Starter::new().start().unwrap();
    let block_id = jcli_wrapper::assert_rest_get_block_tip(&jormungandr.rest_address());
    let actual_hash =
        jcli_wrapper::assert_rest_get_block_by_id(&block_id, &jormungandr.rest_address());

    assert_ne!(&actual_hash, "", "empty block hash");
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_block_id() {
    let incorrect_block_id = "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641aa";
    let jormungandr = Starter::new().start().unwrap();

    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_rest_get_block_command(
            &incorrect_block_id,
            &jormungandr.rest_address(),
        ),
        "node rejected request because of invalid parameters",
    );
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_block_id_in_next_block_id_request() {
    let incorrect_block_id = "e1049ea45726f0b1fc473af54f706546b3331765abf89ae9e6a8333e49621641aa";

    let jormungandr = Starter::new().start().unwrap();

    process_assert::assert_process_failed_and_contains_message(
        jcli_wrapper::jcli_commands::get_rest_get_next_block_id_command(
            &incorrect_block_id,
            &1,
            &jormungandr.rest_address(),
        ),
        "node rejected request because of invalid parameters",
    );
}

#[test]
pub fn test_next_id_is_empty_for_tip_block() {
    let jormungandr = Starter::new().start().unwrap();

    let block_id = jcli_wrapper::assert_rest_get_block_tip(&jormungandr.rest_address());
    let mut next_block_id =
        jcli_wrapper::assert_rest_get_next_block_id(&block_id, &1, &jormungandr.rest_address());

    if next_block_id != "" {
        next_block_id = jcli_wrapper::assert_rest_get_next_block_id(
            &next_block_id,
            &1,
            &jormungandr.rest_address(),
        );
    }

    assert_eq!(&next_block_id, "", "next id for tip block should be empty");
}
