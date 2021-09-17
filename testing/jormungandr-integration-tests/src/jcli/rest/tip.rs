use jormungandr_testing_utils::testing::{configuration::NodeConfigBuilder, jcli::JCli, jormungandr::starter::Starter};

#[test]
pub fn test_correct_id_is_returned_for_block_tip_if_only_genesis_block_exists() {
    let jcli: JCli = Default::default();
    let jormungandr = Starter::new().start().unwrap();
    let block_id = jcli.rest().v0().tip(jormungandr.rest_uri());

    assert_ne!(&block_id, "", "empty block hash");
}

#[test]
pub fn test_correct_error_is_returned_for_incorrect_path() {
    let jcli: JCli = Default::default();
    let config = NodeConfigBuilder::new().build();
    let incorrect_uri = format!("http://{}/api/api", config.rest.listen);

    jcli.rest()
        .v0()
        .tip_expect_fail(incorrect_uri, "tcp connect error");
}
