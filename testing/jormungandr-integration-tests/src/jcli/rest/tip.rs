use crate::common::{
    configuration::NodeConfigBuilder, jcli_wrapper, jormungandr::starter::Starter, process_assert,
};

#[test]
pub fn test_correct_id_is_returned_for_block_tip_if_only_genesis_block_exists() {
    let jormungandr = Starter::new().start().unwrap();
    let block_id = jcli_wrapper::assert_rest_get_block_tip(&jormungandr.rest_uri());

    assert_ne!(&block_id, "", "empty block hash");
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_path() {
    let config = NodeConfigBuilder::new().build();
    let incorrect_uri = format!("http://{}/api/api", config.rest.listen);

    process_assert::assert_process_failed_and_matches_message(
        jcli_wrapper::jcli_commands::get_rest_block_tip_command(&incorrect_uri),
        "tcp connect error",
    );
}
